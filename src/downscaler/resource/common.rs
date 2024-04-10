use kube::client::Client;
use log::info;
use serde_json::{json, Map, Value};
use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use crate::{
    downscaler::{Resources, ScaledResources},
    parser::dynamic_resource_type,
    ScaleState,
};

use crate::error::Error;
use tracing::error;

pub struct ScalingMachinery {
    pub(crate) tobe_replicas: Option<i32>,
    pub(crate) original_replicas: String,
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) annotations: Option<BTreeMap<String, String>>,
    pub(crate) resource_type: Resources,
    pub(crate) scale_state: Arc<ScaleState>,
}

impl ScalingMachinery {
    fn should_downscale(&self) -> bool {
        if let Some(annotations) = self.annotations.as_ref() {
            if let Some(is_downscaled) = annotations.get("kubesaver.com/is_downscaled") {
                if is_downscaled == "false" {
                    return true;
                }
            }
        }
        false
    }

    fn should_upscale(&self) -> Option<i32> {
        if let Some(annotations) = self.annotations.as_ref() {
            if let Some(is_downscaled) = annotations.get("kubesaver.com/is_downscaled") {
                if is_downscaled == "true" {
                    if let Some(original_count) = annotations.get("kubesaver.com/original_count") {
                        if let Ok(scale_up) = original_count.parse::<i32>() {
                            return Some(scale_up);
                        }
                    }
                }
            }
        }
        None
    }

    async fn action_for_downscale(&self, c: Client) -> Result<Option<ScaledResources>, Error> {
        info!("downscaling {} : {}", &self.resource_type, &self.name);
        let patch_result = self
            .patching(
                c.clone(),
                &self.original_replicas,
                self.tobe_replicas,
                "true",
                self.scale_state.clone(),
            )
            .await?;
        Ok(Some(patch_result))
    }

    async fn action_for_upscale(
        &self,
        c: Client,
        scale_up: i32,
    ) -> Result<Option<ScaledResources>, Error> {
        info!("upscaling {} : {}", &self.resource_type, &self.name);
        let patch_result = self
            .patching(
                c.clone(),
                &scale_up.to_string(),
                Some(scale_up),
                "false",
                self.scale_state.clone(),
            )
            .await?;
        Ok(Some(patch_result))
    }

    fn should_downscale_first_time(&self) -> bool {
        self.annotations.is_none()
            || self
                .annotations
                .as_ref()
                .map_or(true, |a| a.get("kubesaver.com/is_downscaled").is_none())
    }

    pub async fn scaling_machinery(
        &self,
        c: Client,
        is_uptime: bool,
    ) -> Result<Option<ScaledResources>, Error> {
        // check if the resource has an annotation kubesaver.com/ignore:"true"
        if let Some(ignore_annotations) = self
            .annotations
            .as_ref()
            .and_then(|a| a.get("kubesaver.com/ignore"))
        {
            if ignore_annotations.eq("true") {
                return Ok(None);
            }
        }
        if !is_uptime {
            if self.should_downscale_first_time() {
                info!("downscaling {} : {}", &self.resource_type, &self.name);
                let patch_result = self
                    .patching(
                        c.clone(),
                        &self.original_replicas,
                        self.tobe_replicas,
                        "true",
                        self.scale_state.clone(),
                    )
                    .await?;
                return Ok(Some(patch_result));
            } else if self.should_downscale() {
                return self.action_for_downscale(c.clone()).await;
            }
        } else if let Some(scale_up) = self.should_upscale() {
            return self.action_for_upscale(c, scale_up).await;
        }
        Ok(None)
    }

    async fn patching(
        &self,
        client: Client,
        orig_count: &str,
        replicas: Option<i32>,
        is_downscale: &str,
        scaled_state: Arc<ScaleState>,
    ) -> Result<ScaledResources, Error> {
        let mut flux_sync = "enabled";
        if is_downscale == "true" {
            flux_sync = "disabled"
        }

        let annotations: Value = json!({
            "annotations": {
                "kubesaver.com/is_downscaled": is_downscale,
                "kubesaver.com/original_count": orig_count,
                "kustomize.toolkit.fluxcd.io/reconcile": flux_sync,
            }
        });

        let spec = match self.resource_type {
            Resources::Deployment | Resources::Namespace | Resources::StatefulSet => {
                json!({ "replicas": replicas.unwrap_or(0) })
            }
            Resources::Hpa => {
                json!({ "minReplicas": replicas.unwrap_or(1) }) // minReplicas should >=1
            }
            Resources::CronJob => {
                json!(
                     {
                        "suspend": is_downscale.parse::<bool>().unwrap()
                    }
                )
            }
        };

        let mut patch = Map::new();
        patch.insert("metadata".to_string(), annotations);
        patch.insert("spec".to_string(), spec);
        let patch_object = Value::Object(patch);

        let rs = dynamic_resource_type(client, &self.namespace, self.resource_type);
        //TODO: Error handling
        if let Some(rs) = rs {
            if let Err(e) = rs.patch_resource(&self.name, &patch_object).await {
                error!("failed to patch resource {}, {}", self.resource_type, e);
                metrics_incrementer(
                    (
                        ScaleType::from_str(is_downscale).unwrap(),
                        ScaleStatus::Failed,
                    ),
                    scaled_state,
                )
            } else {
                metrics_incrementer(
                    (
                        ScaleType::from_str(is_downscale).unwrap(),
                        ScaleStatus::Success,
                    ),
                    scaled_state,
                )
            }
        };
        Ok(ScaledResources {
            name: self.name.to_owned(),
            namespace: self.namespace.to_owned(),
            kind: self.resource_type,
        })
    }
}

fn metrics_incrementer(status: (ScaleType, ScaleStatus), s: Arc<ScaleState>) {
    match status {
        (ScaleType::ScaleUp, ScaleStatus::Success) => s.scaleup_succcess_counter.inc(),
        (ScaleType::ScaleUp, ScaleStatus::Failed) => s.scaleup_error_counter.inc(),
        (ScaleType::ScaleDown, ScaleStatus::Success) => s.scaledown_succcess_counter.inc(),
        (ScaleType::ScaleDown, ScaleStatus::Failed) => s.scaledown_error_counter.inc(),
    }
}

enum ScaleType {
    ScaleUp,
    ScaleDown,
}

enum ScaleStatus {
    Success,
    Failed,
}

impl FromStr for ScaleType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq("true") {
            Ok(ScaleType::ScaleDown)
        } else if s.eq("false") {
            Ok(ScaleType::ScaleUp)
        } else {
            Err(Error::UserInputError(
                "Scale type must be either true or false".to_string(),
            ))
        }
    }
}

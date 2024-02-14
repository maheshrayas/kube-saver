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
    pub async fn scaling_machinery(
        &self,
        c: Client,
        is_uptime: bool,
    ) -> Result<Option<ScaledResources>, Error> {
        if !is_uptime {
            // check if the resource has annotations
            if self.annotations.is_none()
                || self
                    .annotations
                    .to_owned()
                    .unwrap()
                    .get("kubesaver.com/is_downscaled")
                    .is_none()
            {
                // first time action
                info!("downscaling {} : {}", &self.resource_type, &self.name,);
                return Ok(Some(
                    self.patching(
                        c.clone(),
                        &self.original_replicas,
                        self.tobe_replicas,
                        "true",
                        self.scale_state.clone(),
                    )
                    .await?,
                ));
            } else if let Some(x) = self
                .annotations
                .as_ref()
                .unwrap()
                .get("kubesaver.com/is_downscaled")
            {
                // if the resources are already upscaled by the kube-saver and now its the time to be downscaled
                if x == "false" {
                    info!("downscaling {} : {}", &self.resource_type, &self.name);
                    return Ok(Some(
                        self.patching(
                            c.clone(),
                            &self.original_replicas,
                            self.tobe_replicas,
                            "true",
                            self.scale_state.clone(),
                        )
                        .await?,
                    ));
                }
            }
        } else {
            // its a uptime
            // should be up and running
            //  check if annotation is true
            let y = self.annotations.as_ref().unwrap();
            if let Some(x) = y.get("kubesaver.com/is_downscaled") {
                let scale_up: i32 = y
                    .get("kubesaver.com/original_count")
                    .unwrap()
                    .parse()
                    .unwrap();
                if x == "true" {
                    info!("upscaling {} : {} ", &self.resource_type, &self.name);
                    // this is needed becoz the next day I want to downscale after the end time
                    return Ok(Some(
                        self.patching(
                            c.clone(),
                            &scale_up.to_string(), // after scaleup, keep the kubesaver.com/original_count as the real non-zero count.
                            Some(scale_up),
                            "false",
                            self.scale_state.clone(),
                        )
                        .await?,
                    ));
                }
            }
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

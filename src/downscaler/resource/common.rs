use crate::{Error, ResourceExtension, Resources};
use k8s_openapi::api::{
    apps::v1::Deployment, apps::v1::StatefulSet, autoscaling::v1::HorizontalPodAutoscaler,
    batch::v1::CronJob,
};
use kube::{client::Client, Api};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use tracing::info;

pub struct ScalingMachinery {
    pub(crate) tobe_replicas: Option<i32>,
    pub(crate) original_replicas: String,
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) annotations: Option<BTreeMap<String, String>>,
    pub(crate) resource_type: Resources,
}

impl ScalingMachinery {
    pub async fn scaling_machinery(&self, c: Client, is_uptime: bool) -> Result<(), Error> {
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
                info!("downscaling {} {}", &self.resource_type, &self.name,);
                self.patching(
                    c.clone(),
                    &self.original_replicas,
                    self.tobe_replicas,
                    "true",
                )
                .await?;
            } else if let Some(x) = self
                .annotations
                .as_ref()
                .unwrap()
                .get("kubesaver.com/is_downscaled")
            {
                // if the resources are already upscaled by the kube-saver and now its the time to be downscaled
                if x == "false" {
                    info!("downscaling {} {}", &self.resource_type, &self.name);
                    self.patching(
                        c.clone(),
                        &self.original_replicas,
                        self.tobe_replicas,
                        "true",
                    )
                    .await?;
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
                    info!("upscaling {} ", &self.name);
                    // this is needed becoz the next day I want to downscale after the end time
                    self.patching(
                        c.clone(),
                        &scale_up.to_string(), // after scaleup, keep the kubesaver.com/original_count as the real non-zero count.
                        Some(scale_up),
                        "false",
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    async fn patching(
        &self,
        client: Client,
        orig_count: &str,
        replicas: Option<i32>,
        is_downscale: &str,
    ) -> Result<(), Error> {
        let annotations: Value = json!({
            "annotations": {
                "kubesaver.com/is_downscaled": is_downscale,
                "kubesaver.com/original_count": orig_count
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

        let rs: Option<Box<dyn ResourceExtension + Send + Sync>> = match &self.resource_type {
            Resources::Deployment => Some(Box::new(Api::<Deployment>::namespaced(
                client.clone(),
                &self.namespace,
            ))),
            Resources::StatefulSet => Some(Box::new(Api::<StatefulSet>::namespaced(
                client.clone(),
                &self.namespace,
            ))),
            Resources::CronJob => Some(Box::new(Api::<CronJob>::namespaced(
                client.clone(),
                &self.namespace,
            ))),
            Resources::Namespace => None,
            Resources::Hpa => Some(Box::new(Api::<HorizontalPodAutoscaler>::namespaced(
                client.clone(),
                &self.namespace,
            ))),
        };
        match rs {
            Some(rs) => rs.patch_resource(&self.name, &patch_object).await,
            None => Ok(()),
        }
    }
}

use k8s_openapi::api::{
    apps::v1::Deployment, apps::v1::StatefulSet, autoscaling::v1::HorizontalPodAutoscaler,
    batch::v1::CronJob,
};
use kube::{Api, Client};
use log::info;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

use crate::{
    downscaler::{ResourceExtension, Resources},
    util::Error,
};

pub struct UpscaleMachinery {
    pub(crate) replicas: Option<i32>,
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) annotations: Option<BTreeMap<String, String>>,
    pub(crate) resource_type: Resources,
}

impl UpscaleMachinery {
    pub async fn upscale_machinery(&self, c: Client) -> Result<(), Error> {
        let is_annotated = self
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled");
        // before upscaling always crosscheck if the resource is downscaled by kube-saver
        if is_annotated.is_some() {
            let spec = match self.resource_type {
                Resources::Deployment | Resources::Namespace | Resources::StatefulSet => {
                    let replicas = self
                        .get_replicas(self.replicas, self.annotations.to_owned())
                        .await;
                    info!(
                        "scaling up {} in namespace {} to {}",
                        self.name, self.namespace, replicas
                    );
                    json!({ "replicas": replicas })
                }
                Resources::Hpa => {
                    let replicas = self
                        .get_replicas(self.replicas, self.annotations.to_owned())
                        .await;
                    json!({ "minReplicas": replicas }) // minReplicas should >=1
                }

                Resources::CronJob => {
                    info!(
                        "Setting CronJob {} in namespace {} to Active",
                        self.name, self.namespace,
                    );
                    json!(
                         {
                            "suspend": false
                        }
                    )
                }
            };
            let mut patch = Map::new();
            patch.insert("spec".to_string(), spec);
            let patch_object = Value::Object(patch);

            let rs: Option<Box<dyn ResourceExtension>> = match self.resource_type {
                Resources::Deployment => Some(Box::new(Api::<Deployment>::namespaced(
                    c.clone(),
                    &self.namespace,
                ))),
                Resources::StatefulSet => Some(Box::new(Api::<StatefulSet>::namespaced(
                    c.clone(),
                    &self.namespace,
                ))),
                Resources::CronJob => Some(Box::new(Api::<CronJob>::namespaced(
                    c.clone(),
                    &self.namespace,
                ))),
                Resources::Hpa => Some(Box::new(Api::<HorizontalPodAutoscaler>::namespaced(
                    c.clone(),
                    &self.namespace,
                ))),
                Resources::Namespace => None, //nothing to do
            };
            match rs {
                Some(rs) => rs.patch_resource(&self.name, &patch_object).await,
                None => Ok(()),
            }
        } else {
            // do nothing
            Ok(())
        }
    }

    async fn get_replicas(
        &self,
        configured_replicas: Option<i32>,
        annotated_replicas: Option<BTreeMap<String, String>>,
    ) -> i32 {
        let re = if let Some(replicas) = configured_replicas {
            replicas
        } else if let Some(replicas) = annotated_replicas
            .as_ref()
            .unwrap()
            .get("kubesaver.com/original_count")
        {
            replicas.parse().unwrap()
        } else {
            0
        };
        re
    }
}

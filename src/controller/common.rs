use crate::{Error, ResourceExtension, Resources};
use k8s_openapi::api::apps::v1::{Deployment, StatefulSet};
use kube::{Api, Client};
use serde_json::json;
use std::collections::BTreeMap;
use tracing::info;

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
            let replicas = self
                .get_replicas(self.replicas, self.annotations.to_owned())
                .await;
            info!(
                "scaling up {} in namespace {} to {}",
                self.name, self.namespace, replicas
            );
            let patch = json!({
                "spec": {
                    "replicas": replicas
                }
            });

            let rs: Box<dyn ResourceExtension> = match self.resource_type {
                Resources::Deployment => {
                    Box::new(Api::<Deployment>::namespaced(c.clone(), &self.namespace))
                }
                Resources::StatefulSet => {
                    Box::new(Api::<StatefulSet>::namespaced(c.clone(), &self.namespace))
                }
                Resources::Namespace => todo!(),
            };
            Ok(rs.patch_resource(&self.name, &patch).await?)
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

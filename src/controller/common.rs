use kube::Client;
use log::info;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

use crate::error::Error;
use crate::{downscaler::Resources, parser::dynamic_resource_type};

pub struct UpscaleMachinery {
    pub(crate) replicas: Option<i32>,
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) annotations: Option<BTreeMap<String, String>>,
    pub(crate) resource_type: Resources,
}

impl UpscaleMachinery {
    pub async fn upscale_machinery(&self, c: Client) -> Result<(), Error> {
        let annotations = self.annotations.as_ref().unwrap();
        let is_downscaled = annotations.get("kubesaver.com/is_downscaled").is_some();
        // before upscaling always crosscheck if the resource is downscaled by kube-saver
        if is_downscaled {
            let is_flux_disabled = annotations
                .get("kustomize.toolkit.fluxcd.io/reconcile")
                .is_some();
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
            // If "flux" annotation is disabled, remove it
            if is_flux_disabled {
                let annotations: Value = json!({
                    "annotations": {
                        "kustomize.toolkit.fluxcd.io/reconcile": null,
                    }
                });

                patch.insert("metadata".to_string(), annotations);
            }

            let patch_object = Value::Object(patch);
            let rs = dynamic_resource_type(c, &self.namespace, self.resource_type);
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

use crate::controller::common::UpscaleMachinery;
use crate::downscaler::{JMSExpression, Res};
use crate::{Error, ResourceExtension, Resources};
use async_trait::async_trait;
use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler;
use kube::api::{Patch, PatchParams};
use kube::{client::Client, Api};
use serde_json::Value;
use tracing::{debug, info};

use super::common::ScalingMachinery;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Hpa<'a> {
    pub(crate) expression: &'a str,
    pub(crate) replicas: Option<i32>,
    pub(crate) is_uptime: bool,
}

impl JMSExpression for HorizontalPodAutoscaler {}

impl<'a> Hpa<'a> {
    pub fn new(expression: &'a str, replicas: Option<i32>, is_uptime: bool) -> Self {
        Hpa {
            expression,
            replicas,
            is_uptime,
        }
    }
}

#[async_trait]
impl<'a> Res for Hpa<'a> {
    async fn downscale(&self, c: Client) -> Result<(), Error> {
        let api: Api<HorizontalPodAutoscaler> = Api::all(c.clone());
        let list = api.list(&Default::default()).await.unwrap();

        // TODO: Multiple threads
        for item in list.items {
            let result = item.parse(self.expression).await?;
            let original_count = (item.spec.unwrap().min_replicas.unwrap()).to_string();
            if result {
                // if the replicas is set to 0 on the input resource type = 'Namespace', make sure Hpa cannot be set to 0
                // Hence always set it to 1 and the dependent Deployment will be set to 0
                let replicas = if let Some(0) = self.replicas {
                    info!("hpa spec.minReplicas: Invalid value: 0: must be greater than or equal to 1,");
                    Some(1)
                } else {
                    self.replicas
                };
                let pat = ScalingMachinery {
                    tobe_replicas: replicas,
                    original_replicas: original_count,
                    name: item.metadata.name.unwrap(),
                    namespace: item.metadata.namespace.unwrap(),
                    annotations: item.metadata.annotations,
                    resource_type: Resources::Hpa,
                };
                pat.scaling_machinery(c.clone(), self.is_uptime).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ResourceExtension for Api<HorizontalPodAutoscaler> {
    async fn patch_resource(&self, name: &str, patch_value: &Value) -> Result<(), Error> {
        self.patch(name, &PatchParams::default(), &Patch::Merge(patch_value))
            .await?;
        Ok(())
    }

    async fn processor_scale_ns_resource_items(
        &self,
        replicas: Option<i32>,
        c: Client,
        is_uptime: bool,
    ) -> Result<(), Error> {
        let list = self.list(&Default::default()).await?;
        for item in list.items {
            let original_count = (item.spec.unwrap().min_replicas.unwrap()).to_string();
            // if the replicas is set to 0 on the input resource type = 'Namespace', make sure Hpa cannot be set to 0
            // Hence always set it to 1 and the dependent Deployment will be set to 0
            let replicas = if let Some(0) = replicas {
                Some(1)
            } else {
                replicas
            };

            let pat = ScalingMachinery {
                tobe_replicas: replicas,
                original_replicas: original_count,
                name: item.metadata.name.unwrap(),
                namespace: item.metadata.namespace.unwrap(),
                annotations: item.metadata.annotations,
                resource_type: Resources::Hpa,
            };
            pat.scaling_machinery(c.clone(), is_uptime).await?;
        }
        Ok(())
    }

    async fn controller_upscale_resource_items(
        &self,
        replicas: Option<i32>,
        client: Client,
    ) -> Result<(), Error> {
        let hpa_list = self.list(&Default::default()).await.unwrap();
        for cj in &hpa_list.items {
            debug!("parsing hpa resource {:?}", cj.metadata.name);
            // HPA minReplicas cannot be 0
            let replicas = if let Some(0) = replicas {
                Some(1)
            } else {
                replicas
            };
            let u = UpscaleMachinery {
                replicas,
                name: cj.metadata.name.as_ref().unwrap().to_string(),
                namespace: cj.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: cj.metadata.annotations.to_owned(),
                resource_type: Resources::Hpa,
            };
            u.upscale_machinery(client.clone()).await?
        }
        Ok(())
    }
}

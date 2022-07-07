use crate::controller::common::UpscaleMachinery;
use crate::downscaler::{JMSExpression, Res};
use crate::{Error, ResourceExtension, Resources};
use async_trait::async_trait;
use k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler;
use kube::api::{Patch, PatchParams};
use kube::{client::Client, Api};
use serde_json::Value;
use tracing::debug;

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
                let pat = ScalingMachinery {
                    tobe_replicas: self.replicas,
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

    async fn processor_scaler_resource_items(
        &self,
        replicas: Option<i32>,
        c: Client,
        is_uptime: bool,
    ) -> Result<(), Error> {
        let list = self.list(&Default::default()).await?;
        // for item in list.items {
        //     let pat = ScalingMachinery {
        //         tobe_replicas: replicas,
        //         original_replicas: "0".to_string(), // doesn't apply to cronjob
        //         name: item.metadata.name.unwrap(),
        //         namespace: item.metadata.namespace.unwrap(),
        //         annotations: item.metadata.annotations,
        //         resource_type: Resources::CronJob,
        //     };
        //     pat.scaling_machinery(c.clone(), is_uptime).await?;
        // }
        Ok(())
    }

    async fn controller_upscale_resource_items(
        &self,
        replicas: Option<i32>,
        client: Client,
    ) -> Result<(), Error> {
        let hpa_list = self.list(&Default::default()).await.unwrap();
        // for cj in &cj_list.items {
        //     debug!("parsing cronjob resource {:?}", cj.metadata.name);
        //     let u = UpscaleMachinery {
        //         replicas,
        //         name: cj.metadata.name.as_ref().unwrap().to_string(),
        //         namespace: cj.metadata.namespace.as_ref().unwrap().to_string(),
        //         annotations: cj.metadata.annotations.to_owned(),
        //         resource_type: Resources::CronJob,
        //     };
        //     u.upscale_machinery(client.clone()).await?
        // }
        Ok(())
    }
}

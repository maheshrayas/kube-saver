use crate::controller::common::UpscaleMachinery;
use crate::downscaler::{JMSExpression, Res};
use crate::{Error, ResourceExtension, Resources};
use async_trait::async_trait;
use k8s_openapi::api::batch::v1::CronJob;
use kube::api::{Patch, PatchParams};
use kube::{client::Client, Api};
use serde_json::Value;
use tracing::debug;

use super::common::ScalingMachinery;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct CJob<'a> {
    pub(crate) expression: &'a str,
    pub(crate) is_uptime: bool,
}

impl<'a> CJob<'a> {
    pub fn new(expression: &'a str, is_uptime: bool) -> Self {
        CJob {
            expression,
            is_uptime,
        }
    }
}

impl JMSExpression for CronJob {}

#[async_trait]
impl<'a> Res for CJob<'a> {
    async fn downscale(&self, c: Client) -> Result<(), Error> {
        let api: Api<CronJob> = Api::all(c.clone());
        let list = api.list(&Default::default()).await.unwrap();
        // TODO: Multiple threads
        for item in list.items {
            let result = item.parse(self.expression).await?;
            if result {
                let pat = ScalingMachinery {
                    tobe_replicas: None,                // doesn't apply to cronjob
                    original_replicas: "0".to_string(), // doesn't apply to cronjob
                    name: item.metadata.name.unwrap(),
                    namespace: item.metadata.namespace.unwrap(),
                    annotations: item.metadata.annotations,
                    resource_type: Resources::CronJob,
                };
                pat.scaling_machinery(c.clone(), self.is_uptime).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ResourceExtension for Api<CronJob> {
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
            let pat = ScalingMachinery {
                tobe_replicas: replicas,
                original_replicas: "0".to_string(), // doesn't apply to cronjob
                name: item.metadata.name.unwrap(),
                namespace: item.metadata.namespace.unwrap(),
                annotations: item.metadata.annotations,
                resource_type: Resources::CronJob,
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
        let cj_list = self.list(&Default::default()).await.unwrap();
        for cj in &cj_list.items {
            debug!("parsing cronjob resource {:?}", cj.metadata.name);
            let u = UpscaleMachinery {
                replicas,
                name: cj.metadata.name.as_ref().unwrap().to_string(),
                namespace: cj.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: cj.metadata.annotations.to_owned(),
                resource_type: Resources::CronJob,
            };
            u.upscale_machinery(client.clone()).await?
        }
        Ok(())
    }
}

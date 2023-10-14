use std::sync::Arc;

use crate::controller::common::UpscaleMachinery;
use crate::downscaler::{JMSExpression, Res, ResourceExtension, Resources, ScaledResources};
use crate::error::Error;
use crate::ScaleState;
use async_trait::async_trait;
use k8s_openapi::api::batch::v1::CronJob;
use kube::api::{Patch, PatchParams};
use kube::{client::Client, Api};
use log::debug;
use serde_json::Value;

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
    async fn downscale(
        &self,
        c: Client,
        scale_state: Arc<ScaleState>,
    ) -> Result<Vec<ScaledResources>, Error> {
        let api: Api<CronJob> = Api::all(c.clone());
        let list = api.list(&Default::default()).await.unwrap();
        let mut list_cron: Vec<ScaledResources> = vec![];
        // TODO: Multiple threads
        for item in list.items {
            let result = item.parse(self.expression).await?;
            if result {
                let name = item.metadata.name.unwrap();
                let namespace: String = item.metadata.namespace.unwrap();
                let pat = ScalingMachinery {
                    tobe_replicas: None,                // doesn't apply to cronjob
                    original_replicas: "0".to_string(), // doesn't apply to cronjob
                    name,
                    namespace,
                    annotations: item.metadata.annotations,
                    resource_type: Resources::CronJob,
                    scale_state: Arc::clone(&scale_state),
                };
                if let Some(scaled_res) = pat.scaling_machinery(c.clone(), self.is_uptime).await? {
                    list_cron.push(scaled_res);
                }
            }
        }
        Ok(list_cron)
    }
}

#[async_trait]
impl ResourceExtension for Api<CronJob> {
    async fn patch_resource(&self, name: &str, patch_value: &Value) -> Result<(), Error> {
        debug!("patching cronjob resource {:?}", name);
        self.patch(name, &PatchParams::default(), &Patch::Merge(patch_value))
            .await?;
        Ok(())
    }

    async fn processor_scale_ns_resource_items(
        &self,
        replicas: Option<i32>,
        c: Client,
        is_uptime: bool,
        scale_state: Arc<ScaleState>,
    ) -> Result<Vec<ScaledResources>, Error> {
        let list = self.list(&Default::default()).await?;
        let mut list_cron: Vec<ScaledResources> = vec![];
        for item in list.items {
            let name = item.metadata.name.unwrap();
            let namespace = item.metadata.namespace.unwrap();
            debug!(
                "Parsing cronjob {} since its in namespace {:?}",
                name, namespace
            );
            let pat = ScalingMachinery {
                tobe_replicas: replicas,
                original_replicas: "0".to_string(), // doesn't apply to cronjob
                name,
                namespace,
                annotations: item.metadata.annotations,
                resource_type: Resources::CronJob,
                scale_state: Arc::clone(&scale_state),
            };
            if let Some(scaled_res) = pat.scaling_machinery(c.clone(), is_uptime).await? {
                list_cron.push(scaled_res);
            }
        }
        Ok(list_cron)
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

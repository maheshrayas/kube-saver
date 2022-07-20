use crate::controller::common::UpscaleMachinery;
use crate::downscaler::{JMSExpression, Res};
use crate::{Error, ResourceExtension, Resources};
use async_trait::async_trait;
use k8s_openapi::api::apps::v1::Deployment;
use kube::api::{Patch, PatchParams};
use kube::{client::Client, Api};
use log::debug;
use serde_json::Value;

use super::common::ScalingMachinery;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Deploy<'a> {
    pub(crate) expression: &'a str,
    pub(crate) replicas: Option<i32>,
    pub(crate) is_uptime: bool,
}

impl<'a> Deploy<'a> {
    pub fn new(expression: &'a str, replicas: Option<i32>, is_uptime: bool) -> Self {
        Deploy {
            expression,
            replicas,
            is_uptime,
        }
    }
}

impl JMSExpression for Deployment {}

#[async_trait]
impl<'a> Res for Deploy<'a> {
    //TODO: logging
    //TODO: proper error handling
    async fn downscale(&self, c: Client) -> Result<(), Error> {
        let api: Api<Deployment> = Api::all(c.clone());
        let list = api.list(&Default::default()).await.unwrap();
        // TODO: Multiple threads
        for item in list.items {
            let result = item.parse(self.expression).await?;
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            if result {
                let pat = ScalingMachinery {
                    tobe_replicas: self.replicas,
                    original_replicas: original_count,
                    name: item.metadata.name.unwrap(),
                    namespace: item.metadata.namespace.unwrap(),
                    annotations: item.metadata.annotations,
                    resource_type: Resources::Deployment,
                };
                pat.scaling_machinery(c.clone(), self.is_uptime).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ResourceExtension for Api<Deployment> {
    async fn patch_resource(&self, name: &str, patch_value: &Value) -> Result<(), Error> {
        debug!("patching deployment: {}", name);
        self.patch(name, &PatchParams::default(), &Patch::Merge(&patch_value))
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
            let name = item.metadata.name.unwrap();
            let namespace = item.metadata.namespace.unwrap();
            debug!(
                "Parsing deployment {} since its in namespace {:?}",
                name, namespace
            );
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            let pat = ScalingMachinery {
                tobe_replicas: replicas,
                original_replicas: original_count,
                name,
                namespace,
                annotations: item.metadata.annotations,
                resource_type: Resources::Deployment,
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
        let deploy_list = self.list(&Default::default()).await.unwrap();
        for deploy in &deploy_list.items {
            debug!("parsing deployment resource {:?}", deploy.metadata.name);
            let u = UpscaleMachinery {
                replicas,
                name: deploy.metadata.name.as_ref().unwrap().to_string(),
                namespace: deploy.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: deploy.metadata.annotations.to_owned(),
                resource_type: Resources::Deployment,
            };
            u.upscale_machinery(client.clone()).await?
        }
        Ok(())
    }
}

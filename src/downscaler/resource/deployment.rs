use std::sync::Arc;

use crate::controller::common::UpscaleMachinery;
use crate::downscaler::{JMSExpression, Res, ResourceExtension, Resources, ScaledResources};
use crate::error::Error;
use crate::ScaleState;
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


#[allow(clippy::needless_lifetimes)]
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
#[allow(clippy::needless_lifetimes)]
impl<'a> Res for Deploy<'a> {
    //TODO: proper error handling
    async fn downscale(
        &self,
        c: Client,
        scale_state: Arc<ScaleState>,
    ) -> Result<Vec<ScaledResources>, Error> {
        let api: Api<Deployment> = Api::all(c.clone());
        let list = api.list(&Default::default()).await.unwrap();
        let mut list_dep: Vec<ScaledResources> = vec![];
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
                    scale_state: Arc::clone(&scale_state),
                };
                if let Some(scaled_res) = pat.scaling_machinery(c.clone(), self.is_uptime).await? {
                    list_dep.push(scaled_res);
                };
            }
        }
        Ok(list_dep)
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
        scale_state: Arc<ScaleState>,
    ) -> Result<Vec<ScaledResources>, Error> {
        let list = self.list(&Default::default()).await?;
        let mut list_dep: Vec<ScaledResources> = vec![];
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
                scale_state: Arc::clone(&scale_state),
            };
            if let Some(scaled_res) = pat.scaling_machinery(c.clone(), is_uptime).await? {
                list_dep.push(scaled_res);
            };
        }
        Ok(list_dep)
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

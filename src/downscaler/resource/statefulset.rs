use async_trait::async_trait;
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client};
use serde_json::Value;
use tracing::debug;

use crate::controller::common::UpscaleMachinery;
use crate::downscaler::Res;
use crate::{Error, JMSExpression, ResourceExtension, Resources};

use super::common::ScalingMachinery;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct StateSet<'a> {
    pub(crate) expression: &'a str,
    pub(crate) replicas: Option<i32>,
    pub(crate) is_uptime: bool,
}

impl<'a> StateSet<'a> {
    pub fn new(expression: &'a str, replicas: Option<i32>, is_uptime: bool) -> Self {
        StateSet {
            expression,
            replicas,
            is_uptime,
        }
    }
}

impl JMSExpression for StatefulSet {}

#[async_trait]
impl Res for StateSet<'_> {
    async fn downscale(&self, c: Client) -> Result<(), Error> {
        let api: Api<StatefulSet> = Api::all(c.clone());
        let ss = api.list(&Default::default()).await.unwrap();
        for item in ss.items {
            let result = item.parse(self.expression).await?;
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            if result {
                let pat = ScalingMachinery {
                    tobe_replicas: self.replicas,
                    original_replicas: original_count,
                    name: item.metadata.name.unwrap(),
                    namespace: item.metadata.namespace.unwrap(),
                    annotations: item.metadata.annotations,
                    resource_type: Resources::StatefulSet,
                };
                pat.scaling_machinery(c.clone(), self.is_uptime).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ResourceExtension for Api<StatefulSet> {
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
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            let pat = ScalingMachinery {
                tobe_replicas: replicas,
                original_replicas: original_count,
                name: item.metadata.name.unwrap(),
                namespace: item.metadata.namespace.unwrap(),
                annotations: item.metadata.annotations,
                resource_type: Resources::StatefulSet,
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
        let ss_list = self.list(&Default::default()).await.unwrap();
        for ss in &ss_list.items {
            debug!("parsing deployment resource {:?}", ss.metadata.name);
            let u = UpscaleMachinery {
                replicas,
                name: ss.metadata.name.as_ref().unwrap().to_string(),
                namespace: ss.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: ss.metadata.annotations.to_owned(),
                resource_type: Resources::StatefulSet,
            };
            u.upscale_machinery(client.clone()).await?
        }
        Ok(())
    }
}

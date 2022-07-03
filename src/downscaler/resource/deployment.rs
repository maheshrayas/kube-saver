use crate::downscaler::{JMSExpression, Res};
use crate::{Error, Resources};
use async_trait::async_trait;
use k8s_openapi::api::apps::v1::*;
use kube::{client::Client, Api};

use super::common::ScalingMachinery;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Deploy<'a> {
    pub(crate) expression: &'a str,
    pub(crate) replicas: i32,
    pub(crate) is_uptime: bool,
}

impl<'a> Deploy<'a> {
    pub fn new(expression: &'a str, replicas: i32, is_uptime: bool) -> Self {
        Deploy {
            expression,
            replicas,
            is_uptime,
        }
    }
}

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

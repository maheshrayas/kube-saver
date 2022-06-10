use crate::downscaler::{JMSExpression, Res};
use crate::Error;
use async_trait::async_trait;
use k8s_openapi::api::apps::v1::*;
use kube::{client::Client, Api};

use super::common::DeploymentMachinery;

#[derive(Debug, PartialEq, Default)]
pub struct Deploy<'a> {
    pub(crate) expression: &'a str,
    pub(crate) replicas: i32,
}

impl<'a> Deploy<'a> {
    pub fn new() -> Deploy<'a> {
        Deploy {
            ..Default::default()
        }
    }
}

impl JMSExpression for Deployment {}

#[async_trait]
impl<'a> Res for Deploy<'a> {
    //TODO: logging
    //TODO: proper error handling
    async fn downscale(&self, c: Client, is_uptime: bool) -> Result<(), Error> {
        let api: Api<Deployment> = Api::all(c.clone());
        let list = api.list(&Default::default()).await.unwrap();
        // TODO: Multiple threads
        for item in list.items {
            let result = item.parse(self.expression).await?;
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            if result {
                let pat = DeploymentMachinery {
                    tobe_replicas: self.replicas,
                    original_replicas: original_count,
                    name: item.metadata.name.unwrap(),
                    namespace: item.metadata.namespace.unwrap(),
                    annotations: item.metadata.annotations,
                };
                pat.deployment_machinery(c.clone(), is_uptime).await?;
            }
        }
        Ok(())
    }
}

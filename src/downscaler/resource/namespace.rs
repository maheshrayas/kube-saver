use crate::downscaler::{JMSExpression, Res};
use crate::Error;
use async_trait::async_trait;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Namespace;
use kube::api::{Patch, PatchParams};
use kube::{client::Client, Api};
use serde_json::json;
use tracing::info;

use super::common::DeploymentMachinery;

#[derive(Debug, PartialEq, Default)]
pub struct Nspace<'a> {
    pub(crate) expression: &'a str,
    pub(crate) replicas: i32,
}

impl<'a> Nspace<'a> {
    pub fn new() -> Nspace<'a> {
        Nspace {
            ..Default::default()
        }
    }
}

impl JMSExpression for Namespace {}

#[async_trait]
impl<'a> Res for Nspace<'a> {
    //TODO: logging
    //TODO: proper error handling
    async fn downscale(&self, c: Client, is_uptime: bool) -> Result<(), Error> {
        let api: Api<Namespace> = Api::all(c.clone());
        let namespaces = api.list(&Default::default()).await.unwrap();
        // TODO: Multiple threads
        for ns in namespaces.items {
            let result = ns.parse(self.expression).await?;
            if result {
                    let api: Api<Deployment> = Api::namespaced(c.clone(), &ns.metadata.name.unwrap() );
                    let list = api.list(&Default::default()).await.unwrap();
                    for item in list.items {
                        let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
                        if result {
                            let pat = DeploymentMachinery {
                                tobe_replicas: self.replicas,
                                original_replicas:original_count,
                                name: item.metadata.name.unwrap(),
                                namespace: item.metadata.namespace.unwrap(),
                                annotations: item.metadata.annotations
                            };
                            pat.deployment_machinery(c.clone(),is_uptime).await?;
                        }
                }
            }
        }
        Ok(())
    }
}


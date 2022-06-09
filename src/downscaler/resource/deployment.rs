use crate::downscaler::{JMSExpression, Res};
use crate::Error;
use async_trait::async_trait;
use k8s_openapi::api::apps::v1::*;
use kube::api::{Patch, PatchParams};
use kube::{client::Client, Api};
use serde_json::json;
use tracing::info;

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
            let ann = item.metadata.annotations;
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            if result {
                if !is_uptime {
                    // first time action
                    if ann.to_owned().unwrap().get("is_downscaled").is_none() {
                        info!("downscaling {:?}", &item.metadata.name.to_owned().unwrap());
                        //TODO: replicacount should be configured
                        patching(
                            c.clone(),
                            original_count,
                            self.replicas,
                            &item.metadata.name.unwrap(),
                            &item.metadata.namespace.unwrap(),
                            "true",
                        )
                        .await?;
                    } else if let Some(x) = ann.unwrap().get("is_downscaled") {
                        // if the resources are already upscaled by the kube-saver and now its the time to be downscaled
                        if x == "false" {
                            info!("downscaling {:?}", &item.metadata.name.to_owned().unwrap());
                            patching(
                                c.clone(),
                                original_count,
                                self.replicas,
                                &item.metadata.name.unwrap(),
                                &item.metadata.namespace.unwrap(),
                                "true",
                            )
                            .await?;
                        }
                    }
                } else {
                    // its a uptime
                    // should be up and running
                    //  check if annotation is true
                    let y = ann.unwrap();
                    if let Some(x) = y.get("is_downscaled") {
                        let scale_up: i32 = y.get("original_count").unwrap().parse().unwrap();
                        if x == "true" {
                            // this is needed becoz the next day I want to downscale after the end time
                            patching(
                                c.clone(),
                                original_count,
                                scale_up,
                                &item.metadata.name.unwrap(),
                                &item.metadata.namespace.unwrap(),
                                "false",
                            )
                            .await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

//TODO: Struct method
//TODO: use the same method in crd and looper
async fn patching(
    client: Client,
    orig_count: String,
    replicas: i32,
    name: &str,
    namespace: &str,
    is_downscale: &str,
) -> Result<(), Error> {
    let patch = json!({
        "metadata": {
            "annotations": {
                "is_downscaled": is_downscale,
                "original_count": orig_count
            },
        },
        "spec": {
            "replicas": replicas
        }
    });
    let patch_params = PatchParams::default();
    let path_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    path_api
        .patch(name, &patch_params, &Patch::Merge(&patch))
        .await
        .unwrap();
    Ok(())
}

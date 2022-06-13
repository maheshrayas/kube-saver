use crate::Error;
use k8s_openapi::api::apps::v1::*;
use kube::api::{Patch, PatchParams};
use kube::{client::Client, Api};
use serde_json::json;
use std::collections::BTreeMap;
use tracing::info;

pub struct DeploymentMachinery {
    pub(crate) tobe_replicas: i32,
    pub(crate) original_replicas: String,
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) annotations: Option<BTreeMap<String, String>>,
}

//TODO: Struct method
//TODO: use the same method in crd and looper
async fn patching(
    client: Client,
    orig_count: &str,
    replicas: i32,
    name: &str,
    namespace: &str,
    is_downscale: &str,
) -> Result<(), kube::Error> {
    let patch = json!({
        "metadata": {
            "annotations": {
                "kubesaver.com/is_downscaled": is_downscale,
                "kubesaver.com/original_count": orig_count
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

impl DeploymentMachinery {
    pub async fn deployment_machinery(&self, c: Client, is_uptime: bool) -> Result<(), Error> {
        if !is_uptime {
            // first time action
            if self
                .annotations
                .to_owned()
                .unwrap()
                .get("kubesaver.com/is_downscaled")
                .is_none()
            {
                info!("downscaling {:?}", &self.name);
                patching(
                    c.clone(),
                    &self.original_replicas,
                    self.tobe_replicas,
                    &self.name,
                    &self.namespace,
                    "true",
                )
                .await?;
            } else if let Some(x) = self
                .annotations
                .as_ref()
                .unwrap()
                .get("kubesaver.com/is_downscaled")
            {
                // if the resources are already upscaled by the kube-saver and now its the time to be downscaled
                if x == "false" {
                    info!("downscaling {:?}", &self.name);
                    patching(
                        c.clone(),
                        &self.original_replicas,
                        self.tobe_replicas,
                        &self.name,
                        &self.namespace,
                        "true",
                    )
                    .await?;
                }
            }
        } else {
            // its a uptime
            // should be up and running
            //  check if annotation is true
            let y = self.annotations.as_ref().unwrap();
            if let Some(x) = y.get("kubesaver.com/is_downscaled") {
                let scale_up: i32 = y
                    .get("kubesaver.com/original_count")
                    .unwrap()
                    .parse()
                    .unwrap();
                if x == "true" {
                    info!("Upscaling {:?}", &self.name);
                    // this is needed becoz the next day I want to downscale after the end time
                    patching(
                        c.clone(),
                        &scale_up.to_string(), // after scaleup, keep the kubesaver.com/original_count as the real non-zero count.
                        scale_up,
                        &self.name,
                        &self.namespace,
                        "false",
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }
}

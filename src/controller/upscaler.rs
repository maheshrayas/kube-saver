use crate::downscaler::JMSExpression;
use crate::Error;
use k8s_openapi::api::apps::v1::Deployment;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client};
use serde_json::json;
use std::collections::BTreeMap;
use tracing::{debug, info};

/// where `n` is the number of `replicas` given.
///
/// # Arguments
/// - `client` - A Kubernetes client to create the deployment with.
/// - `name` - Name of the deployment to be created
/// - `replicas` - Number of pod replicas for the Deployment to contain
/// - `namespace` - Namespace to create the Kubernetes Deployment in.
///
/// Note: Upscale the resource to original count
pub async fn upscale(
    client: Client,
    name: &str,
    replicas: Option<i32>,
    tags: &BTreeMap<String, String>,
    namespace: &str,
) -> Result<(), Error> {
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());
    let api: Api<Deployment> = Api::all(client.clone());
    let list = api.list(&Default::default()).await?;
    for (key, value) in tags {
        let exp = format!(r#"{}=='{}'"#, key, value);
        debug!("parsing jmes exp {}", exp);
        for item in &list.items {
            debug!("parsing deployment resource {:?}", item.metadata.name);
            let result = item.parse(&exp).await?;
            let ann = item.metadata.annotations.to_owned();
            if result {
                info!(
                    "current count of resource {}",
                    item.spec.as_ref().unwrap().replicas.unwrap()
                );
                let repl = if let Some(replicas) = replicas {
                    replicas
                } else {
                    ann.as_ref()
                        .unwrap()
                        .get("kubesaver.com/original_count")
                        .unwrap()
                        .parse()?
                };

                let patch = json!({
                    "spec": {
                        "replicas": repl
                    }
                });
                let patch_params = PatchParams::default();
                let path_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
                info!(
                    "scaling deployment resource {:?} to original count {}",
                    item.metadata.name, repl
                );
                let _z = path_api
                    .patch(
                        item.metadata.name.as_ref().unwrap(),
                        &patch_params,
                        &Patch::Merge(&patch),
                    )
                    .await?;
            }
        }
    }
    Ok(())
}

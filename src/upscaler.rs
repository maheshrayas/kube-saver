use jmespath::Variable;
use k8s_openapi::api::apps::v1::Deployment;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Error};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Creates a new deployment of `n` pods with the `inanimate/echo-server:latest` docker image inside,
/// where `n` is the number of `replicas` given.
///
/// # Arguments
/// - `client` - A Kubernetes client to create the deployment with.
/// - `name` - Name of the deployment to be created
/// - `replicas` - Number of pod replicas for the Deployment to contain
/// - `namespace` - Namespace to create the Kubernetes Deployment in.
///
/// Note: It is assumed the resource does not already exists for simplicity. Returns an `Error` if it does.
pub async fn deploy(
    client: Client,
    name: &str,
    replicas: Option<i32>,
    tags: &BTreeMap<String, String>,
    namespace: &str,
) -> Result<(), Error> {
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());
    // TODO: Create a trait and impl that trait for the generic type Deployment
    let api: Api<Deployment> = Api::all(client.clone());
    let list = api.list(&Default::default()).await.unwrap();

    // read the tags

    for (key, value) in tags {
        let exp = format!(r#"{}=='{}'"#, key, value);
        println!("exp {}", exp);
        for item in &list.items {
            println!("Deployment {:?}", item.metadata.name);
            let expr = jmespath::compile(&exp).unwrap().to_owned();
            let ann = item.metadata.annotations.to_owned();
            let sr = serde_json::to_string(&item).unwrap();
            let data = jmespath::Variable::from_json(&sr).unwrap();
            let result = expr.search(data).unwrap();

            // let result: Option<bool> = Some(true);
            if result.as_boolean().unwrap() {
                println!("result {} ", result);
                // TODO: check if downscaled
                // get the original count original_count
                if ann.to_owned().unwrap().get("original_count").is_some() {
                    let replicas: i32 = ann
                        .to_owned()
                        .unwrap()
                        .get("original_count")
                        .unwrap()
                        .parse()
                        .unwrap();

                    println!("annotations {}", replicas);
                    println!("namespace {}", namespace);
                    let patch = json!({
                        "spec": {
                            "replicas": replicas
                        }
                    });
                    let patch_params = PatchParams::default();
                    let path_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
                    let z = path_api
                        .patch(
                            &item.metadata.name.as_ref().unwrap(),
                            &patch_params,
                            &Patch::Merge(&patch),
                        )
                        .await
                        .unwrap();

                    //     .unwrap();
                }
            }
        }
    }

    Ok(())
}

/// Deletes an existing deployment.
///
/// # Arguments:
/// - `client` - A Kubernetes client to delete the Deployment with
/// - `name` - Name of the deployment to delete
/// - `namespace` - Namespace the existing deployment resides in
///
/// Note: It is assumed the deployment exists for simplicity. Otherwise returns an Error.
pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
    let api: Api<Deployment> = Api::namespaced(client, namespace);
    println!("I am here in delete");
    // api.delete(name, &DeleteParams::default()).await?;
    Ok(())
}

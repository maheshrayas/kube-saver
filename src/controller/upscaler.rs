use crate::downscaler::JMSExpression;
use crate::Error;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Namespace;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client};
use serde_json::json;
use std::collections::BTreeMap;
use tracing::{debug, info};

pub async fn upscale_deploy(
    client: Client,
    replicas: Option<i32>,
    tags: &BTreeMap<String, String>,
) -> Result<(), Error> {
    let api: Api<Deployment> = Api::all(client.clone());
    let list = api.list(&Default::default()).await?;
    for (key, value) in tags {
        let exp = format!(r#"{}=='{}'"#, key, value);
        debug!("parsing jmes exp {}", exp);
        for item in &list.items {
            debug!("parsing deployment resource {:?}", item.metadata.name);
            let result = item.parse(&exp).await?;
            // before upscaling always crosscheck if the resource is downscaled by kube-saver
            let is_annotated = item
                .metadata
                .annotations
                .as_ref()
                .unwrap()
                .get("kubesaver.com/is_downscaled");
            if result && is_annotated.is_some() {
                let upscale_replicas =
                    get_replicas(replicas, item.metadata.annotations.to_owned()).await;
                patch_up_deployment(
                    client.clone(),
                    upscale_replicas,
                    item.metadata.name.as_ref().unwrap(),
                    item.metadata.namespace.as_ref().unwrap(),
                )
                .await?
            }
        }
    }
    Ok(())
}

pub async fn upscale_ns(
    client: Client,
    replicas: Option<i32>,
    tags: &BTreeMap<String, String>,
) -> Result<(), Error> {
    let api: Api<Namespace> = Api::all(client.clone());
    let namespaces = api.list(&Default::default()).await.unwrap();
    for (key, value) in tags {
        let exp = format!(r#"{}=='{}'"#, key, value);
        debug!("parsing jmes exp {}", exp);
        for ns in &namespaces.items {
            let result = ns.parse(&exp).await?;
            if result {
                // upscale deployment
                let api: Api<Deployment> =
                    Api::namespaced(client.clone(), ns.metadata.name.as_ref().unwrap());
                let deploy_list = api.list(&Default::default()).await.unwrap();
                for deploy in &deploy_list.items {
                    debug!("parsing deployment resource {:?}", deploy.metadata.name);
                    // before upscaling always crosscheck if the resource is downscaled by kube-saver
                    let is_annotated = deploy
                        .metadata
                        .annotations
                        .as_ref()
                        .unwrap()
                        .get("kubesaver.com/is_downscaled");
                    if is_annotated.is_some() {
                        let upscale_replicas =
                            get_replicas(replicas, deploy.metadata.annotations.to_owned()).await;
                        patch_up_deployment(
                            client.clone(),
                            upscale_replicas,
                            deploy.metadata.name.as_ref().unwrap(),
                            deploy.metadata.namespace.as_ref().unwrap(),
                        )
                        .await?
                    }
                }
                //TODO statefulset
            }
        }
    }
    Ok(())
}

async fn patch_up_deployment(
    client: Client,
    replicas: i32,
    name: &str,
    namespace: &str,
) -> Result<(), Error> {
    info!(
        "scaling up deployment {} in namespace {} to {}",
        name, namespace, replicas
    );
    let patch = json!({
        "spec": {
            "replicas": replicas
        }
    });
    let patch_params = PatchParams::default();
    let path_api: Api<Deployment> = Api::namespaced(client, namespace);
    let _z = path_api
        .patch(name, &patch_params, &Patch::Merge(&patch))
        .await?;
    Ok(())
}

async fn get_replicas(
    configured_replicas: Option<i32>,
    annotated_replicas: Option<BTreeMap<String, String>>,
) -> i32 {
    let re = if let Some(replicas) = configured_replicas {
        replicas
    } else if let Some(replicas) = annotated_replicas
        .as_ref()
        .unwrap()
        .get("kubesaver.com/original_count")
    {
        replicas.parse().unwrap()
    } else {
        0
    };
    re
}

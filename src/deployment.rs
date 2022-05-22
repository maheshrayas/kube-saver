use k8s_openapi::api::apps::v1::*;
use kube::api::{Patch, PatchParams};
use kube::Error;
use kube::{client::Client, Api};
use serde_json::json;

//TODO: Struct method
// TODO: use the same method in crd and looper
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

//TODO: make this generic
//TODO: logging
//TODO: proper error handling
pub async fn deployment(client: Client, z: bool) -> Result<(), Error> {
    let api: Api<Deployment> = Api::all(client.clone());
    let list = api.list(&Default::default()).await.unwrap();
    // TODO: Multiple threads
    for item in list.items {
        //let exp = format!(r#"{}=='{}'"#, key, value);
        // TODO : Dont hardcode
        let expr = jmespath::compile("metadata.labels.app == 'go-app'").unwrap();
        let str = serde_json::to_string(&item).unwrap();
        let data = jmespath::Variable::from_json(&str).unwrap();
        let result = expr.search(data).unwrap();
        let ann = item.metadata.annotations;
        let original_count = (*&item.spec.unwrap().replicas.unwrap()).to_string();
        if result.as_boolean().unwrap() {
            if z {
                // first time action
                if ann.to_owned().unwrap().get("is_downscaled").is_none() {
                    println!("downscaling {:?}", &item.metadata.name.to_owned().unwrap());
                    //TODO: replica should be configured
                    patching(
                        client.clone(),
                        original_count,
                        0,
                        &item.metadata.name.to_owned().unwrap(),
                        &item.metadata.namespace.to_owned().unwrap(),
                        "true",
                    )
                    .await?;
                } else {
                    if let Some(x) = ann.unwrap().get("is_downscaled") {
                        if x == "false" {
                            println!("downscaling {:?}", &item.metadata.name.to_owned().unwrap());
                            patching(
                                client.clone(),
                                original_count,
                                0,
                                &item.metadata.name.to_owned().unwrap(),
                                &item.metadata.namespace.to_owned().unwrap(),
                                "true",
                            )
                            .await?;
                        }
                    }
                }
            } else {
                // should be up and running
                //  check if annotation is true
                let y = ann.to_owned().unwrap();
                if let Some(x) = y.get("is_downscaled") {
                    let scale_up: i32 =
                        y.get("original_count").to_owned().unwrap().parse().unwrap();
                    if x == "true" {
                        // this is needed becoz the next day I want to downscale after the end time
                        patching(
                            client.clone(),
                            original_count,
                            scale_up,
                            &item.metadata.name.to_owned().unwrap(),
                            &item.metadata.namespace.to_owned().unwrap(),
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

use core::time;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{api::ListParams, Api, Client};
use kube_saver::controller::upscaler::{upscale_deploy, upscale_ns};
use kube_saver::{JMSExpression, Rules};
use std::{collections::BTreeMap, fs::File, thread};

#[tokio::test]
async fn test1_namespace() {
    let f = File::open("tests/rules/rules1.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone()).await;
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber1");
    let d = api.get("test-kuber1-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber2");
    let d = api.get("test-kuber2-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
}

#[tokio::test]
async fn test3_deployment() {
    let f = File::open("tests/rules/rules2.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone()).await;
    // kube-saver must scale down to 0
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber3");
    let d = api.get("test-kuber3-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    // should be always up
    let d = api.get("test-kuber3-deploy2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
}

#[tokio::test]
async fn test4_apply_upscaler_on_downscaled_for_deployment() {
    let f = File::open("tests/rules/rules3.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone()).await;
    // kube-saver must scale down to 0
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber4");
    let d = api.get("test-kuber4-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let d = api.get("test-kuber4-deploy2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let mut upscale_tags = BTreeMap::new();
    upscale_tags.insert(
        "metadata.name".to_string(),
        "test-kuber4-deploy1".to_string(),
    );
    upscale_deploy(client.clone(), None, &upscale_tags).await;
    // kubectl apply upscaler.yaml
    // // Upsale CR must scale up test-kuber4-deploy1 to 2
    let d = api.get("test-kuber4-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
    let d = api.get("test-kuber4-deploy2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
}

#[tokio::test]
async fn test5_apply_upscaler_on_downscaled_for_namespace() {
    let f = File::open("tests/rules/rules4.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone()).await;
    // kube-saver must scale down to 0
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber5");
    let d = api.get("test-kuber5-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let d = api.get("test-kuber5-deploy2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let mut upscale_tags = BTreeMap::new();
    upscale_tags.insert("metadata.name".to_string(), "kuber5".to_string());
    upscale_ns(client.clone(), None, &upscale_tags).await;
    // kubectl apply upscaler.yaml
    // // Upsale CR must scale up test-kuber4-deploy1 to 2
    let d = api.get("test-kuber5-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
    let d = api.get("test-kuber5-deploy2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
}

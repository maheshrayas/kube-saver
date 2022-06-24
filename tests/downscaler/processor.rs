use core::time;
use k8s_openapi::api::apps::v1::{Deployment, StatefulSet};
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
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber1");
    let d = api.get("test-kuber1-ss1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber2");
    let d = api.get("test-kuber2-ss2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(1));
}

#[tokio::test]
async fn test2_deployment() {
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
async fn test2_statefulset() {
    let f = File::open("tests/rules/rules5.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone()).await;
    // kube-saver must scale down to 0
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber3");
    let d = api.get("test-kuber3-ss1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    // should be always up
    let d = api.get("test-kuber3-ss2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(1));
}

use k8s_openapi::api::apps::v1::Deployment;
use kube::{api::ListParams, Api, Client};
use kube_saver::{JMSExpression, Rules};
use std::fs::File;

#[tokio::test]
async fn test1_namespace_deploy_scale_down_downtime() {
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

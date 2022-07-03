use k8s_openapi::api::{
    apps::v1::{Deployment, StatefulSet},
    batch::v1::CronJob,
};
use kube::{Api, Client};
use kube_saver::Rules;
use std::fs::File;

#[tokio::test]
async fn test1_namespace() {
    let f = File::open("tests/rules/rules1.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone()).await;
    // test if all Deployment are downscaled in namespace
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber1");
    let d = api.get("test-kuber1-deploy1").await.unwrap();
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled")
            .unwrap(),
        "true"
    );
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/original_count")
            .unwrap(),
        "2"
    );
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    // test if all Deployment in kuber2 are not downscaled
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber2");
    let d = api.get("test-kuber2-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled"),
        None
    );
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/original_count"),
        None
    );
    // test if all StatefulSet are downscaled in namespace
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber1");
    let d = api.get("test-kuber1-ss1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled")
            .unwrap(),
        "true"
    );
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/original_count")
            .unwrap(),
        "1"
    );
    // test if all StatefulSet in kuber2 are not downscaled
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber2");
    let d = api.get("test-kuber2-ss2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(1));
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled"),
        None
    );
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/original_count"),
        None
    );
    // test if all Jobs are disabled in namespace
    let api: Api<CronJob> = Api::namespaced(client.clone(), "kuber1");
    let d = api.get("test-kuber1-cj1").await.unwrap();
    assert_eq!(d.spec.unwrap().suspend.unwrap(), true);
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled")
            .unwrap(),
        "true"
    );
    // test if all Jobs are NOT disabled in namespace
    let api: Api<CronJob> = Api::namespaced(client.clone(), "kuber2");
    let d = api.get("test-kuber2-cj2").await.unwrap();
    assert_eq!(d.spec.unwrap().suspend.unwrap(), false);
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled"),
        None
    );
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

#[tokio::test]
async fn test3_cronjob() {
    let f = File::open("tests/rules/rules9.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone()).await;
    // kube-saver must suspend the cronjob
    let api: Api<CronJob> = Api::namespaced(client.clone(), "kuber9");
    let d = api.get("test-kuber9-cj1").await.unwrap();
    assert_eq!(d.spec.unwrap().suspend.unwrap(), true);
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled")
            .unwrap(),
        "true"
    );
    //  // kube-saver must NOT suspend the cronjob
    let d = api.get("test-kuber9-cj2").await.unwrap();
    assert_eq!(d.spec.unwrap().suspend.unwrap(), false);
    assert_eq!(
        d.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled"),
        None
    );
}

#[tokio::test]
async fn test2_scaledown_scaledupresource() {
    let f = File::open("tests/rules/rules7.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone()).await;
    // kube-saver must scale down to 0
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber7");
    let d = api.get("test-kuber7-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
}

#[tokio::test]
async fn test2_scaleup_scaleddownresource() {
    let f = File::open("tests/rules/rules8.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber8");
    let d = api.get("test-kuber8-deploy1").await.unwrap();
    //initially should be zero
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    r.process_rules(client.clone()).await;
    // kube-saver must scale down to 0
    let d = api.get("test-kuber8-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
}

use core::time;
use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler;
use k8s_openapi::api::{
    apps::v1::{Deployment, StatefulSet},
    batch::v1::CronJob,
};
use kube::{Api, Client};
use lazy_static::lazy_static;
use saver::downscaler::Rules;
use saver::ScaleState;
use std::fs::File;
use std::sync::Arc;

lazy_static! {
    pub static ref SCALED_STATE: Arc<ScaleState> = Arc::new(ScaleState::new());
}

#[tokio::test]
async fn test1_namespace() {
    let f = File::open("tests/rules/rules1.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    // sleep for 10 sec so that hpa will scale the replicas original count =3 since hpa
    tokio::time::sleep(time::Duration::from_millis(10000)).await;

    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
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
        "3"
    );
    // this deployment is behind the HPA(minReplicas =3), but if you set the deployment.spec.replicas=0, Hpa should be disabled
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
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
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
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
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
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
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
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
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
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    // kube-saver must scale down to 0
    let d = api.get("test-kuber8-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
}

#[tokio::test]
async fn test4_hpa() {
    let f = File::open("tests/rules/rules12c.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    // kube-saver must set minReplicas =1 in the hpa
    let api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), "kuber12c");
    let d = api.get("test-kuber12c-hpa1").await.unwrap();
    assert_eq!(d.spec.unwrap().min_replicas, Some(1));
    let e = api.get("test-kuber12c-hpa2").await.unwrap();
    assert_eq!(e.spec.unwrap().min_replicas, Some(1));
    // HPA should scale down the respective Deployment to 1
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
        e.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled")
            .unwrap(),
        "true"
    );
}

#[tokio::test]
async fn test4_hpa_scale_all_resources_replicas_1() {
    let f = File::open("tests/rules/rules12.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    //wait for 20 secs for hpa to scale up the deployment
    tokio::time::sleep(time::Duration::from_millis(20000)).await;
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber12");
    let d = api.get("test-kuber12-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(3));

    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    // kube-saver must set minReplicas =1 in the cronjob
    let hpa_api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), "kuber12");
    let hpa = hpa_api.get("test-kuber12-hpa").await.unwrap();
    assert_eq!(hpa.spec.unwrap().min_replicas, Some(1));
    // HPA should scale down the respective Deployment to 1
    assert_eq!(
        hpa.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled")
            .unwrap(),
        "true"
    );
    //wait for 30 secs for hpa to scale down the deployment
    tokio::time::sleep(time::Duration::from_millis(20000)).await;
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber12");
    let d = api.get("test-kuber12-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(1));
    // now test if they are getting scaled up to orignal replicas
    let f = File::open("tests/rules/rules12a.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    let hpa_api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), "kuber12");
    let hpa = hpa_api.get("test-kuber12-hpa").await.unwrap();
    //back to original replicas
    assert_eq!(hpa.spec.unwrap().min_replicas, Some(3));
    assert_eq!(
        hpa.metadata
            .annotations
            .as_ref()
            .unwrap()
            .get("kubesaver.com/is_downscaled")
            .unwrap(),
        "false"
    );
    //wait for 10 secs for hpa to scale up the deployment
    tokio::time::sleep(time::Duration::from_millis(20000)).await;
    let d = api.get("test-kuber12-deploy1").await.unwrap();
    //Deployment must be scaled back to original replicas
    assert_eq!(d.spec.unwrap().replicas, Some(3));
}

#[tokio::test]
async fn test5_check_if_ignored() {
    let f = File::open("tests/rules/rules14.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    // kube-saver must ignore
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber14");
    let d = api.get("test-kuber14-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
}

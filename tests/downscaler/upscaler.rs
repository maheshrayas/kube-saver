use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler;
use k8s_openapi::api::{
    apps::v1::{Deployment, StatefulSet},
    batch::v1::CronJob,
};
use saver::controller::upscaler::{
    enable_cronjob, upscale_deploy, upscale_hpa, upscale_ns, upscale_statefulset,
};
use saver::downscaler::Rules;
use saver::ScaleState;
use std::fs::File;
use std::sync::Arc;

use kube::{api::Api, Client};

use crate::downscaler::processor::SCALED_STATE;

#[tokio::test]
async fn test4_apply_upscaler_on_downscaled_for_deployment() {
    let f = File::open("tests/rules/rules3.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    // kube-saver must scale down to 0
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber4");
    let d = api.get("test-kuber4-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let d = api.get("test-kuber4-deploy2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let exp = "metadata.name=='test-kuber4-deploy1'";

    upscale_deploy(client.clone(), None, exp).await.ok();
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
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    // kube-saver must scale down to 0
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber5");
    let d = api.get("test-kuber5-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let d = api.get("test-kuber5-deploy2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));

    let api: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber5");
    let d = api.get("test-kuber5-ss1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));

    let c: Api<CronJob> = Api::namespaced(client.clone(), "kuber5");
    let c_api = c.get("test-kuber5-cj1").await.unwrap();
    assert_eq!(c_api.spec.unwrap().suspend.unwrap(), true);

    let exp = "metadata.name=='kuber5'";
    upscale_ns(client.clone(), None, exp).await.ok();
    // kubectl apply upscaler.yaml
    // // Upsale CR must scale up test-kuber4-deploy1 to 2
    let api: Api<Deployment> = Api::namespaced(client.clone(), "kuber5");
    let d = api.get("test-kuber5-deploy1").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
    let d = api.get("test-kuber5-deploy2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(2));
    let s: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber5");
    let s_api = s.get("test-kuber5-ss1").await.unwrap();
    assert_eq!(s_api.spec.unwrap().replicas, Some(1));
    let c: Api<CronJob> = Api::namespaced(client.clone(), "kuber5");
    let c_api = c.get("test-kuber5-cj1").await.unwrap();
    assert_eq!(c_api.spec.unwrap().suspend.unwrap(), false);
}

#[tokio::test]
async fn test5_apply_upscaler_on_downscaled_for_statefulset() {
    let f = File::open("tests/rules/rules6.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber6");
    let d = api.get("test-kuber6-ss2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(0));
    let exp = "metadata.name=='test-kuber6-ss2'";
    upscale_statefulset(client.clone(), None, exp).await.ok();
    let api: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber6");
    let d = api.get("test-kuber6-ss2").await.unwrap();
    assert_eq!(d.spec.unwrap().replicas, Some(1));
}

#[tokio::test]
async fn test5_apply_upscaler_on_downscaled_for_cj() {
    let f = File::open("tests/rules/rules10.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    let api: Api<CronJob> = Api::namespaced(client.clone(), "kuber10");
    let c_api = api.get("test-kuber10-cj1").await.unwrap();
    assert_eq!(c_api.spec.unwrap().suspend.unwrap(), true);
    let c_api = api.get("test-kuber10-cj2").await.unwrap();
    assert_eq!(c_api.spec.unwrap().suspend.unwrap(), true);
    let exp = "metadata.name=='test-kuber10-cj1' || metadata.name=='test-kuber10-cj2'";
    enable_cronjob(client.clone(), exp).await.ok();
    let api: Api<CronJob> = Api::namespaced(client.clone(), "kuber10");
    let c_api = api.get("test-kuber10-cj1").await.unwrap();
    assert_eq!(c_api.spec.unwrap().suspend.unwrap(), false);
    let c_api = api.get("test-kuber10-cj2").await.unwrap();
    assert_eq!(c_api.spec.unwrap().suspend.unwrap(), false);
}

#[tokio::test]
async fn test5_apply_upscaler_on_downscaled_for_hpa() {
    let f = File::open("tests/rules/rules12b.yaml").unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(client.clone(), None, None, SCALED_STATE.clone())
        .await
        .ok();
    let api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), "kuber12b");
    let hpa_api = api.get("test-kuber12b-hpa1").await.unwrap();
    assert_eq!(hpa_api.spec.unwrap().min_replicas, Some(1));
    let hpa_api = api.get("test-kuber12b-hpa2").await.unwrap();
    assert_eq!(hpa_api.spec.unwrap().min_replicas, Some(1));
    let exp = "metadata.name=='test-kuber12b-hpa1' || metadata.name=='test-kuber12b-hpa2'";
    upscale_hpa(client.clone(), None, exp).await.ok();
    let api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), "kuber12b");
    let h_api = api.get("test-kuber12b-hpa1").await.unwrap();
    assert_eq!(h_api.spec.unwrap().min_replicas, Some(3));
    let h_api = api.get("test-kuber12b-hpa2").await.unwrap();
    assert_eq!(h_api.spec.unwrap().min_replicas, Some(3));
}

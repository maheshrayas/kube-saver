use core::time;

use k8s_openapi::api::{
    apps::v1::{Deployment, StatefulSet},
    batch::v1::CronJob,
};

use anyhow::Result;
use kube::{api::Api, Client};
use std::fs::File;

#[tokio::test]
#[cfg(not(tarpaulin))]
async fn test6_check_if_upscales() -> Result<()> {
    // First downscale all the resources with specific jmespath
    let f = File::open("tests/rules/rules13.yaml").unwrap();
    let r: saver::downscaler::Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    r.process_rules(
        client.clone(),
        None,
        None,
        std::sync::Arc::new(saver::ScaleState::new()),
    )
    .await
    .ok();

    let cj: Api<CronJob> = Api::namespaced(client.clone(), "kuber13");
    let deploy: Api<Deployment> = Api::namespaced(client.clone(), "kuber13");
    let ss: Api<StatefulSet> = Api::namespaced(client.clone(), "kuber13");
    // All the below resources must be scaled down to zero

    let cj_api = cj.get("test-kuber13-cj1").await.unwrap();
    assert_eq!(cj_api.spec.unwrap().suspend.unwrap(), true);

    let deploy_api = deploy.get("test-kuber13-deploy1").await.unwrap();
    assert_eq!(deploy_api.spec.unwrap().replicas, Some(0));

    let ss_api = ss.get("test-kuber13-ss1").await.unwrap();
    assert_eq!(ss_api.spec.unwrap().replicas, Some(0));
    // All the below resources must not be scaled down

    let cj_api = cj.get("test-kuber13-cj2").await.unwrap();
    assert_eq!(cj_api.spec.unwrap().suspend.unwrap(), false);

    let deploy_api = deploy.get("test-kuber13-deploy2").await.unwrap();
    assert_eq!(deploy_api.spec.unwrap().replicas, Some(2));

    let ss_api = ss.get("test-kuber13-ss2").await.unwrap();
    assert_eq!(ss_api.spec.unwrap().replicas, Some(1));

    // kubectl apply the upscale to scale with the condition that was used to scale down
    // "metadata.labels.env =='sit' && metadata.labels.version !='v2'"
    crate::integration::util::kubectl_appy("tests/upscaler/upscaler-scaleup13.yaml")
        .await
        .ok();
    // sleep for 10 sec so that controller can update the replicas
    tokio::time::sleep(time::Duration::from_millis(10000)).await;
    // lets test it with the actual controller in the

    let cj_api = cj.get("test-kuber13-cj1").await.unwrap();
    assert_eq!(cj_api.spec.unwrap().suspend.unwrap(), false);

    let deploy_api = deploy.get("test-kuber13-deploy1").await.unwrap();
    assert_eq!(deploy_api.spec.unwrap().replicas, Some(2));

    let ss_api = ss.get("test-kuber13-ss1").await.unwrap();
    assert_eq!(ss_api.spec.unwrap().replicas, Some(1));
    Ok(())
}

use crate::controller::common::UpscaleMachinery;
use crate::downscaler::JMSExpression;
use crate::{Error, ResourceExtension, Resources};
use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler;
use k8s_openapi::api::{
    apps::v1::Deployment, apps::v1::StatefulSet, batch::v1::CronJob, core::v1::Namespace,
};
use kube::{Api, Client};
use tracing::debug;

/// Upscale the deploy Resource when CustomResource Upscaler is applied to cluster
pub async fn upscale_deploy(
    client: Client,
    replicas: Option<i32>,
    expression: &str,
) -> Result<(), Error> {
    let api: Api<Deployment> = Api::all(client.clone());
    let list = api.list(&Default::default()).await?;
    // parses the tag map object
    for item in &list.items {
        debug!("parsing deployment resource {:?}", item.metadata.name);
        // for the list of all deployment, check if the tag values matches with the specific deployment
        // For example: metadata.labels.app = nginx is matching with the deployment manifest
        // Invoke the trait JMSExpression default parse method. Deployment implements trait JMSExpression
        let result = item.parse(expression).await?;
        if result {
            let u = UpscaleMachinery {
                replicas,
                name: item.metadata.name.as_ref().unwrap().to_string(),
                namespace: item.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: item.metadata.annotations.to_owned(),
                resource_type: Resources::Deployment,
            };
            u.upscale_machinery(client.clone()).await?
        }
    }

    Ok(())
}

/// Upscale the Statefulset Resource when CustomResource Upscaler is applied to cluster
pub async fn upscale_statefulset(
    client: Client,
    replicas: Option<i32>,
    expression: &str,
) -> Result<(), Error> {
    let api: Api<StatefulSet> = Api::all(client.clone());
    let list = api.list(&Default::default()).await?;

    for item in &list.items {
        debug!("parsing statefulset resource {:?}", item.metadata.name);
        // for the list of all statefulset, check if the tag values matches with the specific statefulset
        // For example: metadata.labels.app = nginx is matching with the statefulset manifest
        // Invoke the trait JMSExpression default parse method. Statefulset implements trait JMSExpression
        let result = item.parse(expression).await?;
        if result {
            let u = UpscaleMachinery {
                replicas,
                name: item.metadata.name.as_ref().unwrap().to_string(),
                namespace: item.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: item.metadata.annotations.to_owned(),
                resource_type: Resources::StatefulSet,
            };
            u.upscale_machinery(client.clone()).await?
        }
    }

    Ok(())
}

/// Set CronJob Suspend status to False when CustomResource Upscaler is applied to cluster
pub async fn enable_cronjob(client: Client, expression: &str) -> Result<(), Error> {
    let api: Api<CronJob> = Api::all(client.clone());
    let list = api.list(&Default::default()).await?;
    for item in &list.items {
        debug!("parsing cronjob resource {:?}", item.metadata.name);
        // for the list of all cronjob, check if the tag values matches with the specific cronjob
        // For example: metadata.labels.app = nginx is matching with the cronjob manifest
        // Invoke the trait JMSExpression default parse method. Statefulset implements trait JMSExpression
        let result = item.parse(expression).await?;
        if result {
            let u = UpscaleMachinery {
                replicas: None,
                name: item.metadata.name.as_ref().unwrap().to_string(),
                namespace: item.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: item.metadata.annotations.to_owned(),
                resource_type: Resources::CronJob,
            };
            u.upscale_machinery(client.clone()).await?
        }
    }

    Ok(())
}

/// Set CronJob Suspend status to False when CustomResource Upscaler is applied to cluster
pub async fn upscale_hpa(
    client: Client,
    replicas: Option<i32>,
    expression: &str,
) -> Result<(), Error> {
    let api: Api<HorizontalPodAutoscaler> = Api::all(client.clone());
    let list = api.list(&Default::default()).await?;
    for item in &list.items {
        debug!("parsing hpa resource {:?}", item.metadata.name);
        // for the list of all Hpa, check if the tag values matches with the specific cronjob
        // For example: metadata.labels.app = nginx is matching with the cronjob manifest
        // Invoke the trait JMSExpression default parse method. Statefulset implements trait JMSExpression
        let result = item.parse(expression).await?;
        if result {
            let u = UpscaleMachinery {
                replicas,
                name: item.metadata.name.as_ref().unwrap().to_string(),
                namespace: item.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: item.metadata.annotations.to_owned(),
                resource_type: Resources::Hpa,
            };
            u.upscale_machinery(client.clone()).await?
        }
    }

    Ok(())
}

/// Upscale the Both Deployment & Statefulset Resource in the defined Namepace that matches the expression defined in the `tag` of CR Upscaler resource
pub async fn upscale_ns(
    client: Client,
    replicas: Option<i32>,
    expression: &str,
) -> Result<(), Error> {
    let api: Api<Namespace> = Api::all(client.clone());
    let namespaces = api.list(&Default::default()).await.unwrap();
    for ns in &namespaces.items {
        // for the list of all Namespace, check if the tag values matches with the specific namespace
        // For example: metadata.name = backend is matching with the Namespace manifest
        // Invoke the trait JMSExpression default parse method. Namespace implements trait JMSExpression
        let result = ns.parse(expression).await?;
        if result {
            // upscale hpa
            let hpa_api: Api<HorizontalPodAutoscaler> =
                Api::namespaced(client.clone(), ns.metadata.name.as_ref().unwrap());
            hpa_api
                .controller_upscale_resource_items(replicas, client.clone())
                .await?;
            // upscale deployment
            let dd_api: Api<Deployment> =
                Api::namespaced(client.clone(), ns.metadata.name.as_ref().unwrap());
            dd_api
                .controller_upscale_resource_items(replicas, client.clone())
                .await?;
            //upscale statefulset
            let ss_api: Api<StatefulSet> =
                Api::namespaced(client.clone(), ns.metadata.name.as_ref().unwrap());
            ss_api
                .controller_upscale_resource_items(replicas, client.clone())
                .await?;
            //Set CronJob Suspend status to False
            let cj_api: Api<CronJob> =
                Api::namespaced(client.clone(), ns.metadata.name.as_ref().unwrap());
            cj_api
                .controller_upscale_resource_items(None, client.clone())
                .await?;
        }
    }
    Ok(())
}

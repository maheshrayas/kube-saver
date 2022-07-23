use crate::downscaler::{JMSExpression, Res};
use crate::{Error, ResourceExtension};
use async_trait::async_trait;
use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler;
use k8s_openapi::api::{
    apps::v1::Deployment, apps::v1::StatefulSet, batch::v1::CronJob, core::v1::Namespace,
};
use kube::{client::Client, Api};
use log::debug;
#[derive(Debug, PartialEq, Eq, Default)]
pub struct Nspace<'a> {
    pub(crate) expression: &'a str,
    pub(crate) replicas: Option<i32>,
    pub(crate) is_uptime: bool,
}

impl<'a> Nspace<'a> {
    pub fn new(expression: &'a str, replicas: Option<i32>, is_uptime: bool) -> Self {
        Nspace {
            expression,
            replicas,
            is_uptime,
        }
    }
}

impl JMSExpression for Namespace {}

#[async_trait]
impl<'a> Res for Nspace<'a> {
    async fn downscale(&self, c: Client) -> Result<(), Error> {
        let api: Api<Namespace> = Api::all(c.clone());
        let namespaces = api.list(&Default::default()).await.unwrap();
        // TODO: Multiple threads
        for ns in namespaces.items {
            let result = ns.parse(self.expression).await?;
            if result {
                let namespace_name = ns.metadata.name.unwrap();
                debug!(
                    "Namespace {} is configured in rules, parsing resources to downscale/upscale",
                    namespace_name
                );

                debug!(
                    "Checking if any HPA resources in namespace {}",
                    namespace_name
                );

                let hpa_api: Api<HorizontalPodAutoscaler> =
                    Api::namespaced(c.clone(), &namespace_name);
                hpa_api
                    .processor_scale_ns_resource_items(self.replicas, c.clone(), self.is_uptime)
                    .await?;
                debug!(
                    "Checking if any Deployment resources in namespace {}",
                    namespace_name
                );
                let d_api: Api<Deployment> = Api::namespaced(c.clone(), &namespace_name);
                d_api
                    .processor_scale_ns_resource_items(self.replicas, c.clone(), self.is_uptime)
                    .await?;
                debug!(
                    "Checking if any StatefulSet resources in namespace {}",
                    namespace_name
                );
                let ss_api: Api<StatefulSet> = Api::namespaced(c.clone(), &namespace_name);
                ss_api
                    .processor_scale_ns_resource_items(self.replicas, c.clone(), self.is_uptime)
                    .await?;
                debug!(
                    "Checking if any CronJob resources in namespace {}",
                    namespace_name
                );
                let cj_api: Api<CronJob> = Api::namespaced(c.clone(), &namespace_name);
                cj_api
                    .processor_scale_ns_resource_items(self.replicas, c.clone(), self.is_uptime)
                    .await?;
            }
        }
        Ok(())
    }
}

use crate::downscaler::{JMSExpression, Res};
use crate::{Error, ResourceExtension};
use async_trait::async_trait;
use k8s_openapi::api::{
    apps::v1::Deployment, apps::v1::StatefulSet, batch::v1::CronJob, core::v1::Namespace,
};
use kube::{client::Client, Api};

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
    //TODO: logging
    //TODO: proper error handling
    async fn downscale(&self, c: Client) -> Result<(), Error> {
        let api: Api<Namespace> = Api::all(c.clone());
        let namespaces = api.list(&Default::default()).await.unwrap();
        // TODO: Multiple threads
        for ns in namespaces.items {
            let result = ns.parse(self.expression).await?;
            if result {
                let d_api: Api<Deployment> =
                    Api::namespaced(c.clone(), ns.metadata.name.as_ref().unwrap());
                d_api
                    .processor_scaler_resource_items(self.replicas, c.clone(), self.is_uptime)
                    .await?;

                let ss_api: Api<StatefulSet> =
                    Api::namespaced(c.clone(), ns.metadata.name.as_ref().unwrap());
                ss_api
                    .processor_scaler_resource_items(self.replicas, c.clone(), self.is_uptime)
                    .await?;

                let ss_api: Api<CronJob> =
                    Api::namespaced(c.clone(), ns.metadata.name.as_ref().unwrap());
                ss_api
                    .processor_scaler_resource_items(self.replicas, c.clone(), self.is_uptime)
                    .await?;
            }
        }
        Ok(())
    }
}

use crate::downscaler::{JMSExpression, Res};
use crate::{Error, Resources};
use async_trait::async_trait;
use k8s_openapi::api::batch::v1::CronJob;
use kube::{client::Client, Api};

use super::common::ScalingMachinery;

#[derive(Debug, PartialEq, Default)]
pub struct CJob<'a> {
    pub(crate) expression: &'a str,
    pub(crate) is_uptime: bool,
}

impl<'a> CJob<'a> {
    pub fn new(expression: &'a str, is_uptime: bool) -> Self {
        CJob {
            expression,
            is_uptime,
        }
    }
}

#[async_trait]
impl<'a> Res for CJob<'a> {
    async fn downscale(&self, c: Client) -> Result<(), Error> {
        let api: Api<CronJob> = Api::all(c.clone());
        let list = api.list(&Default::default()).await.unwrap();
        // TODO: Multiple threads
        for item in list.items {
            let result = item.parse(self.expression).await?;
            if result {
                let pat = ScalingMachinery {
                    tobe_replicas: 0,                   // doesn't apply to cronjob
                    original_replicas: "0".to_string(), // doesn't apply to cronjob
                    name: item.metadata.name.unwrap(),
                    namespace: item.metadata.namespace.unwrap(),
                    annotations: item.metadata.annotations,
                    resource_type: Resources::CronJob,
                };
                pat.scaling_machinery(c.clone(), self.is_uptime).await?;
            }
        }
        Ok(())
    }
}

use async_trait::async_trait;
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::{Api, Client};

use crate::downscaler::Res;
use crate::{Error, JMSExpression, Resources};

use super::common::ScalingMachinery;

#[derive(Debug, PartialEq, Default)]
pub struct StateSet<'a> {
    pub(crate) expression: &'a str,
    pub(crate) replicas: i32,
    pub(crate) is_uptime: bool,
}

impl<'a> StateSet<'a> {
    pub fn new(expression: &'a str, replicas: i32, is_uptime: bool) -> Self {
        StateSet {
            expression,
            replicas,
            is_uptime,
        }
    }
}

#[async_trait]
impl Res for StateSet<'_> {
    async fn downscale(&self, c: Client) -> Result<(), Error> {
        let api: Api<StatefulSet> = Api::all(c.clone());
        let ss = api.list(&Default::default()).await.unwrap();
        for item in ss.items {
            let result = item.parse(self.expression).await?;
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            if result {
                let pat = ScalingMachinery {
                    tobe_replicas: self.replicas,
                    original_replicas: original_count,
                    name: item.metadata.name.unwrap(),
                    namespace: item.metadata.namespace.unwrap(),
                    annotations: item.metadata.annotations,
                    resource_type: Resources::StatefulSet,
                };
                pat.scaling_machinery(c.clone(), self.is_uptime).await?;
            }
        }

        Ok(())
    }
}

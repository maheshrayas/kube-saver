use crate::controller::common::UpscaleMachinery;
use crate::resource::common::ScalingMachinery;
use crate::Error;
use async_trait::async_trait;
use k8s_openapi::api::apps::v1::{Deployment, StatefulSet};
use k8s_openapi::api::core::v1::Namespace;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client};
#[cfg(test)]
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use tracing::debug;

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct Rule {
    pub(crate) id: String,
    pub(crate) uptime: String,
    pub(crate) jmespath: String,
    pub(crate) resource: Vec<String>,
    pub(crate) replicas: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Rules {
    pub(crate) rules: Vec<Rule>,
}

#[async_trait]
pub trait JMSExpression {
    async fn parse(&self, expression: &str) -> Result<bool, Error>
    where
        Self: Serialize,
    {
        let expr = jmespath::compile(expression).unwrap();
        let str = serde_json::to_string(&self).unwrap();
        let data = jmespath::Variable::from_json(&str).unwrap();
        let result = expr.search(data).unwrap();
        Ok(result.as_boolean().unwrap())
    }
}

impl JMSExpression for Deployment {}

impl JMSExpression for Namespace {}

impl JMSExpression for StatefulSet {}

#[async_trait]
pub trait Res {
    async fn downscale(&self, c: Client) -> Result<(), Error>;
}

#[async_trait]
pub trait ResourceExtension: Send + Sync {
    async fn patch_resource(&self, name: &str, patch_value: &Value) -> Result<(), Error>;
    // method is implmented by downscaler aka processor
    async fn processor_scaler_resource_items(
        &self,
        replicas: i32,
        client: Client,
        is_uptime: bool,
    ) -> Result<(), Error>;
    // method is implmented by Upscaler controller/operator
    async fn controller_upscale_resource_items(
        &self,
        replicas: Option<i32>,
        client: Client,
    ) -> Result<(), Error>;
}

#[async_trait]
impl ResourceExtension for Api<Deployment> {
    async fn patch_resource(&self, name: &str, patch_value: &Value) -> Result<(), Error> {
        self.patch(name, &PatchParams::default(), &Patch::Merge(&patch_value))
            .await?;
        Ok(())
    }

    async fn processor_scaler_resource_items(
        &self,
        replicas: i32,
        c: Client,
        is_uptime: bool,
    ) -> Result<(), Error> {
        let list = self.list(&Default::default()).await?;
        for item in list.items {
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            let pat = ScalingMachinery {
                tobe_replicas: replicas,
                original_replicas: original_count,
                name: item.metadata.name.unwrap(),
                namespace: item.metadata.namespace.unwrap(),
                annotations: item.metadata.annotations,
                resource_type: Resources::Deployment,
            };
            pat.scaling_machinery(c.clone(), is_uptime).await?;
        }
        Ok(())
    }

    async fn controller_upscale_resource_items(
        &self,
        replicas: Option<i32>,
        client: Client,
    ) -> Result<(), Error> {
        let deploy_list = self.list(&Default::default()).await.unwrap();
        for deploy in &deploy_list.items {
            debug!("parsing deployment resource {:?}", deploy.metadata.name);
            let u = UpscaleMachinery {
                replicas,
                name: deploy.metadata.name.as_ref().unwrap().to_string(),
                namespace: deploy.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: deploy.metadata.annotations.to_owned(),
                resource_type: Resources::Deployment,
            };
            u.upscale_machinery(client.clone()).await?
        }
        Ok(())
    }
}

#[async_trait]
impl ResourceExtension for Api<StatefulSet> {
    async fn patch_resource(&self, name: &str, patch_value: &Value) -> Result<(), Error> {
        self.patch(name, &PatchParams::default(), &Patch::Merge(patch_value))
            .await?;
        Ok(())
    }

    async fn processor_scaler_resource_items(
        &self,
        replicas: i32,
        c: Client,
        is_uptime: bool,
    ) -> Result<(), Error> {
        let list = self.list(&Default::default()).await?;
        for item in list.items {
            let original_count = (item.spec.unwrap().replicas.unwrap()).to_string();
            let pat = ScalingMachinery {
                tobe_replicas: replicas,
                original_replicas: original_count,
                name: item.metadata.name.unwrap(),
                namespace: item.metadata.namespace.unwrap(),
                annotations: item.metadata.annotations,
                resource_type: Resources::StatefulSet,
            };
            pat.scaling_machinery(c.clone(), is_uptime).await?;
        }
        Ok(())
    }

    async fn controller_upscale_resource_items(
        &self,
        replicas: Option<i32>,
        client: Client,
    ) -> Result<(), Error> {
        let ss_list = self.list(&Default::default()).await.unwrap();
        for ss in &ss_list.items {
            debug!("parsing deployment resource {:?}", ss.metadata.name);
            let u = UpscaleMachinery {
                replicas,
                name: ss.metadata.name.as_ref().unwrap().to_string(),
                namespace: ss.metadata.namespace.as_ref().unwrap().to_string(),
                annotations: ss.metadata.annotations.to_owned(),
                resource_type: Resources::StatefulSet,
            };
            u.upscale_machinery(client.clone()).await?
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum Resources {
    Deployment,
    StatefulSet,
    Namespace,
}

impl FromStr for Resources {
    type Err = Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "deployments" | "deployment" => Ok(Resources::Deployment),
            "statefulset"| "statefulsets" => Ok(Resources::StatefulSet),
            "namespace" | "namespaces" => Ok(Resources::Namespace),
            e => Err(Error::UserInputError(format!(
                "Unsupported resource type {}, Currently supports only Deployment, StatefulSet, Namespace (Case Sensitive)",
                e
            ))),
        }
    }
}

impl std::fmt::Display for Resources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Resources::Deployment => write!(f, "Deployment"),
            Resources::StatefulSet => write!(f, "StatefulSet"),
            Resources::Namespace => write!(f, "Namespace"),
        }
    }
}

#[test]
fn test_valid_input_resource_deployment() {
    assert_eq!(
        Resources::from_str("Deployment").unwrap(),
        Resources::Deployment
    );
    assert_eq!(
        Resources::from_str("deployment").unwrap(),
        Resources::Deployment
    );
    assert_eq!(
        Resources::from_str("deployments").unwrap(),
        Resources::Deployment
    );
    assert_eq!(
        Resources::from_str("Deployments").unwrap(),
        Resources::Deployment
    )
}

#[test]
fn test_valid_input_resource_namespace() {
    assert_eq!(
        Resources::from_str("Namespace").unwrap(),
        Resources::Namespace
    );
    assert_eq!(
        Resources::from_str("Namespaces").unwrap(),
        Resources::Namespace
    );
    assert_eq!(
        Resources::from_str("namespace").unwrap(),
        Resources::Namespace
    );
    assert_eq!(
        Resources::from_str("namespaces").unwrap(),
        Resources::Namespace
    );
}
#[test]
fn test_valid_input_resource_statefulset() {
    assert_eq!(
        Resources::from_str("StatefulSet").unwrap(),
        Resources::StatefulSet
    );
    assert_eq!(
        Resources::from_str("StatefulSets").unwrap(),
        Resources::StatefulSet
    );
    assert_eq!(
        Resources::from_str("Statefulset").unwrap(),
        Resources::StatefulSet
    );
    assert_eq!(
        Resources::from_str("Statefulsets").unwrap(),
        Resources::StatefulSet
    );
    assert_eq!(
        Resources::from_str("statefulset").unwrap(),
        Resources::StatefulSet
    );
    assert_eq!(
        Resources::from_str("statefulsets").unwrap(),
        Resources::StatefulSet
    );
}

#[test]
fn test_invalid() {
    let res = Resources::from_str("StatefulSet1");
    assert_eq!(
        res.unwrap_err().to_string(),
        "Invalid User Input: Unsupported resource type statefulset1, Currently supports only Deployment, StatefulSet, Namespace (Case Sensitive)".to_string()
    )
}

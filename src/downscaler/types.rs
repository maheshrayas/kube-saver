use async_trait::async_trait;
use kube::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{str::FromStr, sync::Arc};

use crate::error::Error;

#[derive(Clone)]
pub struct ScaleState {
    pub(crate) scaledown_succcess_counter: prometheus::IntCounter,
    pub(crate) scaleup_succcess_counter: prometheus::IntCounter,
    pub(crate) scaleup_error_counter: prometheus::IntCounter,
    pub(crate) scaledown_error_counter: prometheus::IntCounter,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub(crate) struct Rule {
    pub(crate) id: String,
    pub(crate) uptime: String,
    pub(crate) jmespath: String,
    pub(crate) resource: Vec<String>,
    pub(crate) replicas: Option<i32>,
    pub(crate) slack_channel: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Rules {
    pub(crate) rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct ScaledResources {
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) kind: Resources,
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

#[async_trait]
pub trait Res {
    async fn downscale(&self, c: Client, s: Arc<ScaleState>)
        -> Result<Vec<ScaledResources>, Error>;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Resources {
    Deployment,
    StatefulSet,
    Namespace,
    CronJob,
    Hpa,
}

#[async_trait]
pub trait ResourceExtension: Send + Sync {
    async fn patch_resource(&self, name: &str, patch_value: &Value) -> Result<(), Error>;
    // method is implmented by downscaler aka processor
    async fn processor_scale_ns_resource_items(
        &self,
        replicas: Option<i32>,
        client: Client,
        is_uptime: bool,
        scale_state: Arc<ScaleState>,
    ) -> Result<Vec<ScaledResources>, Error>;
    // method is implmented by Upscaler controller/operator
    async fn controller_upscale_resource_items(
        &self,
        replicas: Option<i32>,
        client: Client,
    ) -> Result<(), Error>;
}

impl FromStr for Resources {
    type Err = Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "deployments" | "deployment" => Ok(Resources::Deployment),
            "statefulset"| "statefulsets" => Ok(Resources::StatefulSet),
            "namespace" | "namespaces" => Ok(Resources::Namespace),
            "cronjob" | "cronjobs" => Ok(Resources::CronJob),
            "hpa" | "horizontalpodautoscaler" | "horizontalpodautoscalers" => Ok(Resources::Hpa),
            e => Err(Error::UserInputError(format!(
                "Unsupported resource type {}, Currently supports only Deployment, StatefulSet, Namespace, Hpa, CronJob",
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
            Resources::CronJob => write!(f, "CronJob"),
            Resources::Hpa => write!(f, "Hpa"),
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
fn test_valid_input_resource_cronjob() {
    assert_eq!(Resources::from_str("CronJob").unwrap(), Resources::CronJob);
    assert_eq!(Resources::from_str("cronjob").unwrap(), Resources::CronJob);
    assert_eq!(Resources::from_str("cronjobs").unwrap(), Resources::CronJob);
    assert_eq!(Resources::from_str("CronJobs").unwrap(), Resources::CronJob);
}

#[test]
fn test_valid_input_resource_hpa() {
    assert_eq!(Resources::from_str("Hpa").unwrap(), Resources::Hpa);
    assert_eq!(Resources::from_str("hpa").unwrap(), Resources::Hpa);
    assert_eq!(
        Resources::from_str("horizontalpodautoscaler").unwrap(),
        Resources::Hpa
    );
    assert_eq!(
        Resources::from_str("horizontalpodautoscalers").unwrap(),
        Resources::Hpa
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
        "Invalid User Input: Unsupported resource type statefulset1, Currently supports only Deployment, StatefulSet, Namespace, Hpa, CronJob".to_string()
    )
}

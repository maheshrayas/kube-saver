use crate::Error;
use async_trait::async_trait;
use kube::Client;
#[cfg(test)]
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

#[async_trait]
pub trait Res {
    async fn downscale(&self, c: Client) -> Result<(), Error>;
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
        match input {
            "Deployment" => Ok(Resources::Deployment),
            "StatefulSet" => Ok(Resources::StatefulSet),
            "Namespace" => Ok(Resources::Namespace),
            e => Err(Error::UserInputError(format!(
                "Unsupported resource type {}",
                e
            ))),
        }
    }
}

#[test]
fn test_valid_input_resource_deployment() {
    let res = Resources::from_str("Deployment");
    assert_eq!(res.unwrap(), Resources::Deployment)
}
#[test]
fn test_valid_input_resource_namespace() {
    let res = Resources::from_str("Namespace");
    assert_eq!(res.unwrap(), Resources::Namespace)
}
#[test]
fn test_valid_input_resource_statefulset() {
    let res = Resources::from_str("StatefulSet");
    assert_eq!(res.unwrap(), Resources::StatefulSet)
}

#[test]
fn test_invalid() {
    let res = Resources::from_str("StatefulSet1");
    assert_eq!(
        res.unwrap_err().to_string(),
        "Invalid User Input: Unsupported resource type StatefulSet1".to_string()
    )
}

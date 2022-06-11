use crate::Error;
use async_trait::async_trait;
use kube::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Rule {
    pub(crate) id: String,
    pub(crate) uptime: String,
    pub(crate) jmespath: String,
    pub(crate) resource: Vec<String>,
    pub(crate) replicas: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Rules {
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

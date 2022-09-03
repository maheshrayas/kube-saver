use chrono::Local;
use env_logger::Env;
use k8s_openapi::api::{
    apps::v1::Deployment, apps::v1::StatefulSet, autoscaling::v1::HorizontalPodAutoscaler,
    batch::v1::CronJob,
};
use kube::{Api, Client};
use log::error;
use std::io::Write;
use std::num::ParseIntError;
use std::str::FromStr;

use crate::downscaler::{ResourceExtension, Resources};

pub fn init_logger() {
    let env = Env::default().filter("RUST_LOG");
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {}: {}",
                Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.args()
            )
        })
        .init();
}

pub fn check_input_resource(r: &str) -> Option<Resources> {
    match Resources::from_str(r) {
        Ok(r) => Some(r),
        Err(err) => {
            // Supported Resource only Deployment, StatefulSet, Namespace, Cronjob, hpa
            error!("{err}");
            // if any one Resource is invalid, dont exit nonzero rather Return None and continue for next rule
            None
        }
    }
}

/// All errors possible to occur during reconciliation
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Any error originating from the `kube-rs` crate
    #[error("Kubernetes reported error: {source}")]
    KubeError {
        #[from]
        source: kube::Error,
    },
    /// Error in user input or typically missing fields.
    #[error("Invalid User Input: {0}")]
    UserInputError(String),
    /// Error in while converting the string to int
    #[error("Invalid Upscaler CRD: {source}")]
    ParseError {
        #[from]
        source: ParseIntError,
    },
    #[error("CSV Error: {source}")]
    CSVError {
        #[from]
        source: csv::Error,
    },

    #[error("IO Error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },

    #[error("Reqwest Error: {source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::UserInputError(s)
    }
}
/// Context injected with each `reconcile` and `on_error` method invocation.
pub struct ContextData {
    pub client: Client,
}

impl ContextData {
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }
}

pub fn dynamic_resource_type(
    c: Client,
    ns: &str,
    resource_type: Resources,
) -> Option<Box<dyn ResourceExtension + Send + Sync>> {
    match resource_type {
        Resources::Deployment => Some(Box::new(Api::<Deployment>::namespaced(c, ns))),
        Resources::StatefulSet => Some(Box::new(Api::<StatefulSet>::namespaced(c, ns))),
        Resources::CronJob => Some(Box::new(Api::<CronJob>::namespaced(c, ns))),
        Resources::Hpa => Some(Box::new(Api::<HorizontalPodAutoscaler>::namespaced(c, ns))),
        Resources::Namespace => None, //nothing to do
    }
}

#[test]
fn test_init_logger() {
    init_logger();
}
#[test]
fn test_input_resource_deployment() {
    let f = check_input_resource("deployment");
    assert_eq!(f, Some(Resources::Deployment));
}
#[test]
fn test_input_resource_hpa() {
    let f = check_input_resource("hpa");
    assert_eq!(f, Some(Resources::Hpa));
}

#[test]
fn test_input_resource_cronjob() {
    let f = check_input_resource("cronjob");
    assert_eq!(f, Some(Resources::CronJob));
}

#[test]
fn test_input_resource_statefulset() {
    let f = check_input_resource("statefulset");
    assert_eq!(f, Some(Resources::StatefulSet));
}

#[test]
fn test_input_resource_namespace() {
    let f = check_input_resource("namespace");
    assert_eq!(f, Some(Resources::Namespace));
}

#[test]
fn test_input_resource_unsupported() {
    let f = check_input_resource("pod");
    assert_eq!(f, None);
}

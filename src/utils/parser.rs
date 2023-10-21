use clap::Parser;
use clap::{error::ErrorKind, CommandFactory};
use k8s_openapi::api::{
    apps::v1::Deployment, apps::v1::StatefulSet, autoscaling::v1::HorizontalPodAutoscaler,
    batch::v1::CronJob,
};
use kube::{Api, Client};
use log::{error, info};
use std::{env, fs, path::Path, str::FromStr};

use crate::error::Error;
use crate::{ResourceExtension, Resources};

const SLACK_TOKEN_FILE_PATH: &str = "/var/slack_token/slack.txt";

#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    // Loop interval in secs
    #[clap(short, long, default_value_t = 60)]
    pub interval: u64,
    /// rules yaml
    #[clap(short, long, default_value = "/config/rules.yaml")]
    pub rules: String,
    /// supply --debug to print the debug information
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
    /// supply --comm_type=slack  to print the debug information
    #[clap(long, value_enum)]
    pub comm_type: Option<CommType>,
    /// supply --comm_details=<slack_org_group>, this arg is mandatory if --comm_type=slack is set
    #[clap(long, value_parser)]
    pub comm_details: Option<String>,
}
impl Args {
    pub fn new() -> Self {
        let cli = Self::parse();
        match cli.debug {
            1 => {
                std::env::set_var("RUST_LOG", "debug,kube_client=off,tower=off,hyper=off");
            }
            _ => {
                std::env::set_var("RUST_LOG", "info,kube_client=off");
            }
        }
        let comm: (Option<CommType>, Option<String>) = if let Some(comm_type) = cli.comm_type {
            let comm_details = cli.comm_details.unwrap_or_else(|| {
                let mut cmd = Self::command();
                cmd.error(
                    ErrorKind::MissingRequiredArgument,
                    "comm-details with required when using --comm-type",
                )
                .exit()
            });
            (Some(comm_type), Some(comm_details))
        } else {
            (None, None)
        };

        Self {
            interval: cli.interval,
            rules: cli.rules,
            debug: cli.debug,
            comm_type: comm.0,
            comm_details: comm.1,
        }
    }
}

impl FromStr for CommType {
    type Err = clap::error::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "slack" | "Slack" => Ok(CommType::Slack),
            e => {
                let mut cmd = Args::command();
                cmd.error(
                    ErrorKind::InvalidValue,
                    format!("{e} is invalid input, Support args are Slack"),
                )
                .exit()
            }
        }
    }
}

pub fn init_logger() {
    // check the rust log
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    // Initialize the logger
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
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

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum CommType {
    Slack,
}

impl CommType {
    pub fn get_secret(&self) -> Result<String, Error> {
        match *self {
            CommType::Slack => {
                let env_slack_token = env::var("SLACK_API_TOKEN");
                let secret_vol = Path::new(SLACK_TOKEN_FILE_PATH);
                // Read ENV variable SLACK_API_TOKEN
                let token = if let Ok(token) = env_slack_token {
                    token
                } else if secret_vol.exists() {
                    // if not present in ENV variable, look for volume
                    let f = fs::read_to_string(secret_vol);
                    if let Ok(slack_token) = f {
                        slack_token
                    } else {
                        error!(
                            "Could not find slack api token at {}",
                            SLACK_TOKEN_FILE_PATH
                        );
                        return Err(Error::MissingRequiredArgument(
                            "Could not find slack api token".to_string(),
                        ));
                    }
                } else {
                    info!("Missing slack api token, either set SLACK_API_TOKEN or mount token as volune at {}", SLACK_TOKEN_FILE_PATH);
                    // log error slack token not found
                    return Err(Error::MissingRequiredArgument(
                        "Could not find slack api token".to_string(),
                    ));
                };
                Ok(token)
            }
        }
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

use chrono_tz::Tz;
use env_logger::Env;
use k8s_openapi::chrono::Local;
use k8s_openapi::chrono::{DateTime, Datelike, TimeZone, Utc};
use kube::Client;
use log::{debug, error};
use regex::Captures;
use std::io::Write;
use std::num::ParseIntError;
use std::process::exit;
use std::str::FromStr;

use crate::downscaler::Resources;

pub fn current_day(day: &str) -> u32 {
    match day {
        "Mon" => 0,
        "Tue" => 1,
        "Wed" => 2,
        "Thu" => 3,
        "Fri" => 4,
        "Sat" => 5,
        "Sun" => 6,
        _ => exit(1),
    }
}

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

pub fn is_uptime(m: Captures) -> Result<bool, Error> {
    let week_start = current_day(&m[1]);
    let week_end = current_day(&m[2]);
    let low_hour: u32 = FromStr::from_str(&m[3]).unwrap();
    let low_min: u32 = FromStr::from_str(&m[4]).unwrap();
    let high_hour: u32 = FromStr::from_str(&m[5]).unwrap();
    let high_min: u32 = FromStr::from_str(&m[6]).unwrap();
    let config_tz: &str = &m["tz"];
    let tz: Tz = config_tz.parse()?;
    // get the current datetime based on the timezone
    let dt: DateTime<Tz> = Utc::now().with_timezone(&tz);
    // get the current time
    let time_of_day = dt.time();
    // check if the current day is configured in the input week range
    if dt.weekday().num_days_from_monday() >= week_start
        && dt.weekday().num_days_from_monday() <= week_end
    {
        let config_date_low_hour = Utc
            .ymd(dt.year(), dt.month(), dt.day())
            .and_hms_milli(low_hour, low_min, 0, 0);
        let config_date_high_hour = Utc
            .ymd(dt.year(), dt.month(), dt.day())
            .and_hms_milli(high_hour, high_min, 0, 0);
        // if the current date time is greater or equal to current date low hour and current date time is less than or equal to current date high hour.
        if time_of_day > config_date_low_hour.time() && time_of_day < config_date_high_hour.time() {
            // the uptime is between the range
            // start upscaling
            debug!("Current rules states, its a uptime for configured resources");
            Ok(true)
        } else {
            // the downtime is between the range
            // start downscaling
            debug!("Current rules states, its a downtime for configured resources");
            Ok(false)
        }
    } else {
        // current day is not configured in the uptime
        debug!("current day is not configured in the uptime,hence downscaling");
        Ok(false)
    }
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

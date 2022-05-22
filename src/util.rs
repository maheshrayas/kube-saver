use chrono::{DateTime, Datelike, TimeZone, Utc};
use kube::Client;
use regex::{Captures, Regex};
use std::process::exit;
use std::str::FromStr;
use tracing::*;

use chrono_tz::Tz;
use kube_runtime::controller::{Action, Context};
use tokio::time::Duration;

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
//TODO: proper error handling
pub fn is_downscale(m: Captures) -> bool {
    let week_start = current_day(&m[1]);
    let week_end = current_day(&m[2]);
    let low_hour: u32 = FromStr::from_str(&m[3]).unwrap();
    let low_min: u32 = FromStr::from_str(&m[4]).unwrap();
    let high_hour: u32 = FromStr::from_str(&m[5]).unwrap();
    let high_min: u32 = FromStr::from_str(&m[6]).unwrap();
    let config_tz = &m["tz"];
    let tz: Tz = config_tz.parse().unwrap();
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
            println!("In the uptime");
            false
        } else {
            // the downtime is between the range
            // start downscaling
            println!("In the downtime");
            true
        }
    } else {
        // current day is not configured in the uptime
        println!("current day is not configured in the uptime,hence downscaling");
        true
    }
}

pub fn is_downscale_time(downscale_time: &str) -> Result<bool, Error> {
    let m = match Regex::new(
        r"^([a-zA-Z]{3})-([a-zA-Z]{3}) (\d\d):(\d\d)-(\d\d):(\d\d) (?P<tz>[a-zA-Z/_]+)$",
    ) {
        Ok(value) => match value.is_match(downscale_time) {
            true => {
                let m = value.captures(downscale_time).unwrap();
                Ok(is_downscale(m))
            }
            false => Ok(false),
        },
        Err(e) => Err(Error::UserInputError(e.to_string())),
    };
    println!("{:?}", m);
    m
}

/// Actions to be taken when a reconciliation fails - for whatever reason.
/// Prints out the error to `stderr` and requeues the resource for another reconciliation after
/// five seconds.
///
/// # Arguments
/// - `error`: A reference to the `kube::Error` that occurred during reconciliation.
/// - `_context`: Unused argument. Context Data "injected" automatically by kube-rs.
pub fn on_error(error: &Error, _context: Context<ContextData>) -> Action {
    eprintln!("Reconciliation error:\n{:?}", error);
    Action::requeue(Duration::from_secs(5))
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
    /// Error in user input or Echo resource definition, typically missing fields.
    #[error("Invalid Echo CRD: {0}")]
    UserInputError(String),
}
/// Context injected with each `reconcile` and `on_error` method invocation.
pub struct ContextData {
    /// Kubernetes client to make Kubernetes API requests with. Required for K8S resource management.
    pub client: Client,
}

impl ContextData {
    /// Constructs a new instance of ContextData.
    ///
    /// # Arguments:
    /// - `client`: A Kubernetes client to make Kubernetes REST API requests with. Resources
    /// will be created and deleted with this client.
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }
}

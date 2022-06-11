use chrono_tz::Tz;
use env_logger::{Builder, Env};
use k8s_openapi::chrono::{DateTime, Datelike, TimeZone, Utc};
use kube::Client;
use regex::{Captures, Regex};
use std::io::Write;
use std::num::ParseIntError;
use std::process::exit;
use std::str::FromStr;
use tracing::*;

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
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,kube_client=off");
    }
    let env = Env::default()
        .filter("RUST_LOG")
        .write_style("MY_LOG_STYLE");

    Builder::from_env(env)
        .format(|buf, record| {
            let style = buf.style();
            // style.set_bg(Color::Yellow).set_bold(true);

            let timestamp = buf.timestamp();

            writeln!(buf, "{}: {}", timestamp, style.value(record.args()))
        })
        .init();
}

pub fn is_uptime(m: Captures) -> bool {
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
            debug!("Current rules states, its a uptime for configured resources");
            true
        } else {
            // the downtime is between the range
            // start downscaling
            debug!("Current rules states, its a downtime for configured resources");
            false
        }
    } else {
        // current day is not configured in the uptime
        debug!("current day is not configured in the uptime,hence downscaling");
        false
    }
}

pub fn validate_uptime(downscale_time: &str) -> Result<bool, Error> {
    let m = match Regex::new(
        r"^([a-zA-Z]{3})-([a-zA-Z]{3}) (\d\d):(\d\d)-(\d\d):(\d\d) (?P<tz>[a-zA-Z/_]+)$",
    ) {
        Ok(value) => match value.is_match(downscale_time) {
            true => {
                let m = value.captures(downscale_time).unwrap();
                Ok(is_uptime(m))
            }
            false => Ok(false),
        },
        Err(e) => Err(Error::UserInputError(e.to_string())),
    };
    m
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
    #[error("Invalid Upscaler CRD: {0}")]
    UserInputError(String),
    /// Error in while converting the string to int
    #[error("Invalid Upscaler CRD: {source}")]
    ParseError {
        #[from]
        source: ParseIntError,
    },
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

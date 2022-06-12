use crate::downscaler::{Res, Resources, Rule, Rules};
use crate::resource::deployment::Deploy;
use crate::resource::namespace::Nspace;
use crate::{is_uptime, Error};
use core::time;
use kube::Client;
#[cfg(test)]
use pretty_assertions::assert_eq;
use regex::Regex;
use std::{fs::File, str::FromStr};
use tracing::{debug, error, info};

pub async fn processor(interval: u64, rules: &str) -> Result<(), Error> {
    let interval_millis = time::Duration::from_millis(interval * 1000);
    let f = File::open(rules).unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default().await?;
    info!(
        "Confgured to look for resource at the interval of {} secs",
        interval_millis.as_secs()
    );
    loop {
        r.process_rules(client.clone()).await?;
        tokio::time::sleep(interval_millis).await;
    }
}

impl Rules {
    pub async fn process_rules(&self, client: Client) -> Result<(), Error> {
        for e in &self.rules {
            info!("Processing rule {}", e.id);
            debug!(
                "Checking if the current timestamp is in the uptime slot {} for the rule id {}",
                e.uptime, e.id
            );
            // check if the resource needs to be up
            let is_uptime = match e.validate_uptime() {
                Ok(is_uptrue) => is_uptrue,
                Err(e) => {
                    error!("{}", e);
                    // just return false rather than panic or non-zero exit
                    false
                }
            };
            debug!("uptime for rule id {} is currently {}", e.uptime, is_uptime);
            // for each resource in rules.yaml
            for r in &e.resource {
                let f = Resources::from_str(r).unwrap();
                info!("Processing rule {} for {}", e.id, r);
                match f {
                    Resources::Deployment => {
                        let d = Deploy::new(&e.jmespath, e.replicas.parse::<i32>()?, is_uptime);
                        d.downscale(client.clone()).await?
                    }
                    Resources::Namespace => {
                        let n = Nspace::new(&e.jmespath, e.replicas.parse::<i32>()?, is_uptime);
                        n.downscale(client.clone()).await?
                    }
                    Resources::StatefulSet => todo!(),
                };
            }
        }
        Ok(())
    }
}

impl Rule {
    /// Returns true if its a uptime
    fn validate_uptime(&self) -> Result<bool, Error> {
        let m = match Regex::new(
            r"^([a-zA-Z]{3})-([a-zA-Z]{3}) (\d\d):(\d\d)-(\d\d):(\d\d) (?P<tz>[a-zA-Z/_]+)$",
        ) {
            Ok(value) => match value.is_match(&self.uptime) {
                true => {
                    let m = value.captures(&self.uptime).unwrap();
                    is_uptime(m)
                }
                false => Ok(false),
            },
            Err(e) => Err(Error::UserInputError(e.to_string())),
        };
        m
    }
}

#[test]
fn validate_invalid_datetime_regex() {
    let r = Rule {
        uptime: String::from("blah"),
        ..Default::default()
    };
    let uptime = r.validate_uptime();
    assert_eq!(uptime.unwrap(), false);
}

#[test]
fn validate_should_be_uptime_regex() {
    let r = Rule {
        uptime: String::from("Mon-Sun 00:00-23:59 Australia/Sydney"),
        ..Default::default()
    };
    let uptime = r.validate_uptime();
    assert_eq!(uptime.unwrap(), true);
}

#[test]
fn validate_invalid_timezone_regex() {
    let r = Rule {
        uptime: String::from("Mon-Sun 00:00-23:59 India/Sydney"),
        ..Default::default()
    };
    let uptime = r.validate_uptime();
    assert_eq!(uptime.is_err(), true);
}

#[test]
fn validate_should_be_downtime_regex() {
    let r = Rule {
        uptime: String::from("Mon-Sun 23:58-23:59 Australia/Sydney"),
        ..Default::default()
    };
    let uptime = r.validate_uptime();
    assert_eq!(uptime.unwrap(), false);
}

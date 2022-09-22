use crate::csv::generate_csv;
use crate::downscaler::resource::{
    cronjob::CJob, deployment::Deploy, hpa::Hpa, namespace::Nspace, statefulset::StateSet,
};
use crate::downscaler::{Res, Resources, Rule, Rules};
use crate::error::Error;
use crate::parser::{check_input_resource, Args, CommType};
use crate::slack::Slack;
use crate::time_check::is_uptime;
use core::time;
use kube::Client;
use log::{debug, error, info};
use regex::Regex;
use std::fs::File;

#[derive(Clone)]
pub struct Process {
    interval: u64,
    rules: String,
    comm_type: Option<CommType>,
    comm_detail: Option<String>,
}

impl From<Args> for Process {
    fn from(k: Args) -> Self {
        Self {
            interval: k.interval,
            rules: k.rules,
            comm_type: k.comm_type,
            comm_detail: k.comm_details,
        }
    }
}

impl Process {
    #[cfg(not(tarpaulin_include))]
    pub async fn processor(&self) -> Result<(), Error> {
        let interval_millis = time::Duration::from_millis(self.interval * 1000);
        let f = File::open(&self.rules).unwrap();
        let r: Rules = serde_yaml::from_reader(f).unwrap();
        let client = Client::try_default().await?;
        info!(
            "Confgured to look for resource at the interval of {} secs",
            interval_millis.as_secs()
        );
        loop {
            let ret = r
                .process_rules(
                    client.clone(),
                    self.comm_type.clone(),
                    self.comm_detail.clone(),
                )
                .await;

            match ret {
                Ok(a) => a,
                Err(e) => {
                    // dont break the loop/process, just report the error to stdout
                    error!("Error: {}", e);
                }
            };
            tokio::time::sleep(interval_millis).await;
        }
    }
}

#[allow(unused_variables)]
impl Rules {
    pub async fn process_rules(
        &self,
        client: Client,
        comm_type: Option<CommType>,
        comm_detail: Option<String>,
    ) -> Result<(), Error> {
        for e in &self.rules {
            debug!(
                "Checking if the current timestamp is in the uptime slot {} for the rule id {}",
                e.uptime, e.id
            );
            // check if the resource needs to be up
            let is_uptime = match e.validate_uptime() {
                Ok(is_uptrue) => is_uptrue,
                Err(er) => {
                    error!("Error while reading rule id {} : {} ", e.id, er);
                    // don't break the loop
                    continue;
                }
            };

            debug!("uptime for rule id {} is currently {}", e.uptime, is_uptime);
            // for each resource in rules.yaml
            for r in &e.resource {
                let f = check_input_resource(r);
                if f.is_some() {
                    info!("Processing rule {} for {}", e.id, r);

                    let resoure_list = match f.unwrap() {
                        Resources::Hpa => {
                            let h = Hpa::new(&e.jmespath, e.replicas, is_uptime);
                            h.downscale(client.clone()).await?
                        }
                        Resources::Deployment => {
                            let d = Deploy::new(&e.jmespath, e.replicas, is_uptime);
                            d.downscale(client.clone()).await?
                        }
                        Resources::Namespace => {
                            let n = Nspace::new(&e.jmespath, e.replicas, is_uptime);
                            n.downscale(client.clone()).await?
                        }
                        Resources::StatefulSet => {
                            let s = StateSet::new(&e.jmespath, e.replicas, is_uptime);
                            s.downscale(client.clone()).await?
                        }
                        Resources::CronJob => {
                            let c = CJob::new(&e.jmespath, is_uptime);
                            c.downscale(client.clone()).await?
                        }
                    };
                    // Send the alert only if resources are scaled down or upped
                    if !resoure_list.is_empty() {
                        if let Some(ref comm) = comm_type {
                            match comm {
                                CommType::Slack => {
                                    generate_csv(&resoure_list, &e.id)?;
                                    let slack_channel = &e.slack_channel;
                                    let token = comm.get_secret().unwrap();
                                    let s = Slack::new(
                                        is_uptime,
                                        e.slack_channel.as_ref().unwrap(),
                                        &e.id,
                                        "KubeSaver Alert",
                                        "maheshrayas",
                                        &token,
                                    );
                                    s.send_slack_msg().await?;
                                }
                            }
                        }
                    }
                }
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
                false => Err(Error::UserInputError(String::from("Input datetime format didn't match <DAY>-<DAY> <START_TIME_HR>:<START_TIME_MIN>-<END_TIME_HR>:<END_TIME_MIN> <TIMEZONE>, Refer sample example in README.md"))),
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
    assert_eq!(
        uptime.unwrap_err().to_string(),
        "Invalid User Input: Input datetime format didn't match <DAY>-<DAY> <START_TIME_HR>:<START_TIME_MIN>-<END_TIME_HR>:<END_TIME_MIN> <TIMEZONE>, Refer sample example in README.md".to_string()
    )
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

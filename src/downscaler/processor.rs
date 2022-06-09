use core::time;
use kube::Client;
use regex::Regex;
use std::{fs::File, str::FromStr};
use tracing::{debug, info};

use crate::downscaler::{Res, Resources, Rule, Rules};
use crate::{is_uptime, Error};

pub async fn processor(interval: u64, rules: &str) -> Result<(), Error> {
    let interval_millis = time::Duration::from_millis(interval * 1000);
    let f = File::open(rules).unwrap();
    let r: Rules = serde_yaml::from_reader(f).unwrap();
    let client = Client::try_default().await?;
    info!(
        "Confgured to look for resource at the interval of {}",
        interval_millis.as_secs()
    );
    loop {
        r.process_rules(client.clone()).await?;
        tokio::time::sleep(interval_millis).await;
    }
}

impl Rules {
    async fn process_rules(&self, client: Client) -> Result<(), Error> {
        for e in &self.rules {
            info!("Processing rule {}", e.id);
            debug!(
                "Checking if the current timestamp is in the uptime slot {} for the rule id {}",
                e.uptime, e.id
            );
            // check if the resource needs to be up
            let is_uptime = e.validate_uptime()?;
            debug!("uptime for rule id {} is currently {}", e.uptime, is_uptime);
            // for each resource in rules.yaml
            for r in &e.resource {
                let f = Resources::from_str(r).unwrap();
                // Static despatch
                match f {
                    Resources::Deployment(mut d) => {
                        info!("Processing rule {} for {}", e.id, r);
                        d.expression = &e.jmespath;
                        d.replicas = e.replicas.parse::<i32>()?;
                        d.downscale(client.clone(), is_uptime).await?
                    }
                    Resources::StatefulSet(s) => s.downscale(client.clone(), is_uptime).await?,
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
                    Ok(is_uptime(m))
                }
                false => Ok(false),
            },
            Err(e) => Err(Error::UserInputError(e.to_string())),
        };
        m
    }
}

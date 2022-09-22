use std::fs;

use crate::error::Error;
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt; // for read_to_end()

const SLACK_UPLOAD_API_ERROR: &str = "https://api.slack.com/methods/files.upload#errors";

#[derive(Debug)]
pub struct Slack<'a> {
    channel: &'a str,
    file_name: &'a str,
    slack_msg_info: &'a str,
    comment: &'a str,
    slack_org: &'a str,
    token: &'a str,
}

#[derive(Deserialize, Serialize)]
pub struct SlackResponse {
    ok: bool,
    error: Option<String>,
}

impl<'a> Slack<'a> {
    pub fn new(
        comment: &'a str,
        channel: &'a str,
        file_name: &'a str,
        slack_msg_info: &'a str,
        slack_org: &'a str,
        token: &'a str,
    ) -> Self {
        Slack {
            channel,
            file_name,
            slack_msg_info,
            comment,
            slack_org,
            token,
        }
    }

    pub async fn send_slack_msg(&self) -> Result<(), Error> {
        let client = reqwest::Client::new();
        let mut file = File::open(format!("{}.csv", self.file_name)).await?;
        let mut contents = vec![];
        let channel = self.channel;
        file.read_to_end(&mut contents).await?;
        let part =
            reqwest::multipart::Part::bytes(contents).file_name(self.slack_msg_info.to_owned());
        let form = reqwest::multipart::Form::new()
            .text("channels", channel.to_owned())
            .text("initial_comment", self.comment.to_owned())
            .part("file", part);

        let response = client
            .post(format!(
                "https://{}.slack.com/api/files.upload",
                self.slack_org
            ))
            .bearer_auth(self.token)
            .multipart(form)
            .send()
            .await;

        // check if the api call is success
        if let Err(e) = response {
            info!("failed to send slack message, check if the slack token is configured correctly");
            Err(Error::ReqwestError { source: e })
        } else {
            let r: SlackResponse = response.unwrap().json().await.unwrap();
            if r.error.is_none() {
                info!(
                    "Sent alert message to slack channel {}, org {}",
                    self.channel, self.slack_org
                );
            } else {
                error!(
                    "Something went wrong while sending slack notification to channel: {}, org: {}, Error: {} \n visit {} for more details on error",
                    self.channel, self.slack_org,r.error.unwrap(), SLACK_UPLOAD_API_ERROR
                );
            }
            Ok(())
        }
    }
}

impl Drop for Slack<'_> {
    fn drop(&mut self) {
        fs::remove_file(format!("{}.csv", self.file_name))
            .unwrap_or_else(|_| info!("failed to delete {}", self.file_name));
    }
}

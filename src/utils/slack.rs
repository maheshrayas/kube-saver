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

#[derive(Deserialize, Serialize, Debug)]
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

    // TODO validate all inputs using proc macro
    fn validate_slack_params(&self) -> Result<(), Error> {
        // if channel is empty
        let _ = self.channel.is_empty();
        // if comment is empty
        let _ = self.comment.is_empty();
        // if file_name is empty
        let _ = self.file_name.is_empty();

        // if slack_msg_info is empty
        let _ = self.slack_msg_info.is_empty();

        // if slack_org is empty
        if self.slack_org.is_empty() {
            return Err(Error::UserInputError(
                "Slack Org cannot be empty".to_string(),
            ));
        }
        // if token is empty
        if self.token.is_empty() {
            return Err(Error::UserInputError(
                "Slack Token cannot be empty".to_string(),
            ));
        };
        Ok(())
    }

    pub async fn send_slack_msg(&self) -> Result<(), Error> {
        self.validate_slack_params()?;
        let url = format!("https://{}.slack.com", self.slack_org);

        let client = reqwest::Client::new();
        let mut file = File::open(format!("/tmp/{}.csv", self.file_name)).await?;
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
            .post(format!("{}/api/files.upload", url))
            .bearer_auth(self.token)
            .multipart(form)
            .send()
            .await;

        // check if the api call is success
        if let Err(e) = response {
            info!("failed to send slack message, check if the slack token is configured correctly");
            Err(Error::ReqwestError { source: e })
        } else {
            let r = response?.json::<SlackResponse>().await?;
            if r.error.is_none() {
                info!(
                    "Sent alert message to slack channel {}, org {}",
                    self.channel, self.slack_org
                );
            } else {
                let slack_response_error_code = r.error.unwrap();
                error!(
                    "Something went wrong while sending slack notification to channel: {}, org: {}, Error: {} \n visit {} for more details on error",
                    self.channel, self.slack_org,slack_response_error_code, SLACK_UPLOAD_API_ERROR
                );
                return Err(Error::SlackResponseError(format!(
                    "Error Code {}",
                    slack_response_error_code
                )));
            }
            Ok(())
        }
    }
}

impl Drop for Slack<'_> {
    fn drop(&mut self) {
        fs::remove_file(format!("/tmp/{}.csv", self.file_name))
            .unwrap_or_else(|_| info!("failed to delete {}", self.file_name));
    }
}

#[tokio::test]
async fn slack_msg_no_csv_file() {
    let s = Slack::new(
        "Scaling Up event completed for rule id rules-downscale-kuber1",
        "CHANNEL",
        "file6",
        "KubeSaverAlert",
        "test",
        "XXXX",
    );
    let retu = s.send_slack_msg().await;
    assert_eq!(
        retu.unwrap_err().to_string(),
        "IO Error: No such file or directory (os error 2)"
    );
}

#[tokio::test]
async fn slack_msg_invalid_response() {
    let s = Slack::new(
        "Scaling Up event completed for rule id rules-downscale-kuber1",
        "CHANNEL",
        "file5",
        "KubeSaverAlert",
        "test",
        "XXXX",
    );
    let _ = File::create("/tmp/file5.csv").await;
    let retu = s.send_slack_msg().await;
    assert_eq!(
        retu.unwrap_err().to_string(),
        "Slack Error: Error Code invalid_auth"
    );
}
#[tokio::test]
async fn slack_msg_null_org() {
    let s = Slack::new(
        "Scaling Up event completed for rule id rules-downscale-kuber1",
        "CHANNEL",
        "file4",
        "KubeSaverAlert",
        " ",
        "XXXX",
    );
    let _ = File::create("/tmp/file4.csv").await;
    let retu = s.send_slack_msg().await;
    assert_eq!(
        retu.unwrap_err().to_string(),
        "Reqwest Error: builder error"
    );
}

#[tokio::test]
async fn slack_msg_empty_org() {
    let s = Slack::new(
        "Scaling Up event completed for rule id rules-downscale-kuber1",
        "CHANNEL",
        "file3",
        "KubeSaverAlert",
        "",
        "XXXX",
    );
    let _ = File::create("/tmp/file3.csv").await;
    let retu = s.send_slack_msg().await;
    assert_eq!(
        retu.unwrap_err().to_string(),
        "Invalid User Input: Slack Org cannot be empty"
    );
}

#[tokio::test]
async fn slack_msg_token_with_newline_or_invalid() {
    let s = Slack::new(
        "Scaling Up event completed for rule id rules-downscale-kuber1",
        "CHANNEL",
        "file2",
        "KubeSaverAlert",
        "test_org",
        "token\n ",
    );
    let _ = File::create("/tmp/file2.csv").await;
    let retu = s.send_slack_msg().await;
    assert_eq!(
        retu.unwrap_err().to_string(),
        "Reqwest Error: builder error"
    );
}

#[tokio::test]
async fn slack_msg_empty_token() {
    let s = Slack::new(
        "Scaling Up event completed for rule id rules-downscale-kuber1",
        "CHANNEL",
        "file1",
        "KubeSaverAlert",
        "test_org",
        "",
    );
    let _ = File::create("/tmp/file1.csv").await;
    let retu = s.send_slack_msg().await;
    assert_eq!(
        retu.unwrap_err().to_string(),
        "Invalid User Input: Slack Token cannot be empty"
    );
}

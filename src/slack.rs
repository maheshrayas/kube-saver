use crate::util::Error;
use tokio::fs::File;
use tokio::io::AsyncReadExt; // for read_to_end()

pub async fn send_slack_msg(channel: String, file_name: &str) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let mut file = File::open(format!("{file_name}.csv")).await?;
    let mut contents = vec![];
    file.read_to_end(&mut contents).await?;
    let part = reqwest::multipart::Part::bytes(contents).file_name("test.csv");
    let form = reqwest::multipart::Form::new()
        .text("channels", channel)
        .text("initial_comment", "Scaling Down")
        .part("file", part);

    let response = client
        .post("https://maheshrayas.slack.com/api/files.upload")
        .header("Authorization", "Bearer ")
        .multipart(form)
        .send()
        .await
        .unwrap();
    println!("response {:?}", response);
    Ok(())
}

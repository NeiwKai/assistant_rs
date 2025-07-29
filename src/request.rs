use reqwest::{Client, Error};
use std::result::Result;

#[tokio::main]
pub async fn request(chat_history: &mut serde_json::Value) -> Result<serde_json::Value, Box<Error>> {
    let client = Client::new(); 
    let res = client.post("http://localhost:8080/v1/chat/completions")
        .json(&chat_history)
        .send()
        .await;
    let result_json: serde_json::Value = match res {
        Ok(res) => match res.json().await {
            Ok(json) => json,
            Err(err) => return Err(Box::new(err)),
        },
        Err(err) => return Err(Box::new(err)),
    };
    Ok(result_json)
}

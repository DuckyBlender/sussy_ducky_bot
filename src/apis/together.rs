use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageRequest {
    pub model: String,
    pub prompt: String,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub n: u32,
    pub response_format: String,
}

pub struct TogetherClient {
    client: Client,
}

#[derive(Deserialize, Debug)]
pub struct ImageResponse {
    // pub id: String,
    // pub model: String,
    // pub object: String,
    pub data: Vec<ImageData>,
}

#[derive(Deserialize, Debug)]
pub struct ImageData {
    pub timings: Timings,
    // pub index: u32,
    pub b64_json: String,
}

#[derive(Deserialize, Debug)]
pub struct Timings {
    pub inference: f64,
}

impl TogetherClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn submit_request(
        &self,
        prompt: ImageRequest,
    ) -> Result<ImageResponse, reqwest::Error> {
        // base64 image

        let response = self
            .client
            .post("https://api.together.xyz/v1/images/generations")
            .header(
                "Authorization",
                format!("Bearer {}", env::var("TOGETHER_KEY").unwrap()),
            )
            .header("Content-Type", "application/json")
            .json(&prompt)
            .send()
            .await?;

        let response = response.json::<ImageResponse>().await?;

        Ok(response)
    }
}
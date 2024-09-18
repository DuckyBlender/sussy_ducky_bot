use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageRequest {
    pub prompt: String,
    pub image_size: String,          // e.g. "landscape_4_3"
    pub num_inference_steps: u32,    // Number of inference steps
    pub num_images: u32,             // Number of images to generate
    pub enable_safety_checker: bool, // Enable safety checker
}

#[derive(Deserialize, Debug)]
pub struct ImageResponse {
    pub request_id: String,
}

#[derive(Deserialize, Debug)]
pub struct ImageStatus {
    pub status: String,
}

#[derive(Deserialize, Debug)]
pub struct ImageResult {
    pub images: Vec<ImageFile>,
}

#[derive(Deserialize, Debug)]
pub struct ImageFile {
    pub url: String,
    // pub content_type: String,
}

pub struct FalClient {
    client: Client,
}

impl FalClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn submit_request(
        &self,
        request: ImageRequest,
    ) -> Result<ImageResponse, reqwest::Error> {
        let response = self
            .client
            .post("https://queue.fal.run/fal-ai/flux/schnell")
            .header(
                "Authorization",
                format!("Key {}", env::var("FAL_KEY").unwrap()),
            )
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .json::<ImageResponse>()
            .await?;

        Ok(response)
    }

    pub async fn check_status(&self, request_id: &str) -> Result<ImageStatus, reqwest::Error> {
        let url = format!("https://queue.fal.run/fal-ai/flux/requests/{request_id}/status");
        let response = self
            .client
            .get(&url)
            .header(
                "Authorization",
                format!("Key {}", env::var("FAL_KEY").unwrap()),
            )
            .send()
            .await?
            .json::<ImageStatus>()
            .await?;

        Ok(response)
    }

    pub async fn get_result(&self, request_id: &str) -> Result<ImageResult, reqwest::Error> {
        let url = format!("https://queue.fal.run/fal-ai/flux/requests/{request_id}");
        let response = self
            .client
            .get(&url)
            .header(
                "Authorization",
                format!("Key {}", env::var("FAL_KEY").unwrap()),
            )
            .send()
            .await?
            .json::<ImageResult>()
            .await?;

        Ok(response)
    }
}

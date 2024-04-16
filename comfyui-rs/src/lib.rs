use futures_util::StreamExt;
use log::{error, info};
use reqwest::Error as ReqwestError;
use serde_json::json;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::io::Error as IoError;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use uuid::Uuid;

#[derive(Debug)]
pub enum ClientError {
    ReqwestError(ReqwestError),
    TungsteniteError(TungsteniteError),
    IoError(IoError),
    JsonError(serde_json::Error),
    CustomError(String),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::ReqwestError(e) => write!(f, "ReqwestError: {}", e),
            ClientError::TungsteniteError(e) => write!(f, "TungsteniteError: {}", e),
            ClientError::IoError(e) => write!(f, "IoError: {}", e),
            ClientError::JsonError(e) => write!(f, "JsonError: {}", e),
            ClientError::CustomError(e) => write!(f, "CustomError: {}", e),
        }
    }
}

impl From<ReqwestError> for ClientError {
    fn from(err: ReqwestError) -> Self {
        ClientError::ReqwestError(err)
    }
}

impl From<TungsteniteError> for ClientError {
    fn from(err: TungsteniteError) -> Self {
        ClientError::TungsteniteError(err)
    }
}

impl From<IoError> for ClientError {
    fn from(err: IoError) -> Self {
        ClientError::IoError(err)
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        ClientError::JsonError(err)
    }
}

impl From<String> for ClientError {
    fn from(err: String) -> Self {
        ClientError::CustomError(err)
    }
}

pub struct Client {
    server_address: String,
    client_id: String,
}

impl Client {
    pub fn new(server_address: &str) -> Self {
        let client_id = Uuid::new_v4().to_string();
        Client {
            server_address: server_address.to_string(),
            client_id,
        }
    }

    pub async fn queue_prompt(
        &self,
        prompt: serde_json::Value,
    ) -> Result<serde_json::Value, ClientError> {
        info!("Queueing prompt");
        let client = reqwest::Client::new();
        let p = json!({
            "prompt": prompt,
            "client_id": self.client_id
        });

        let res = client
            .post(format!("http://{}/prompt", self.server_address))
            .json(&p)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to queue prompt: {}", e);
                ClientError::from(e)
            })?;

        let response: serde_json::Value = res.json().await.map_err(|e| {
            error!("Failed to parse response: {}", e);
            ClientError::from(e)
        })?;
        Ok(response)
    }

    // Similar changes for other methods...

    pub async fn get_images(
        &self,
        json_prompt: serde_json::Value,
    ) -> Result<HashMap<String, Vec<u8>>, ClientError> {
        let prompt_id = self.queue_prompt(json_prompt).await?;

        info!("prompt_id: {prompt_id:?}");

        let prompt_id = prompt_id["prompt_id"].as_str();
        if prompt_id.is_none() {
            return Err(ClientError::CustomError("prompt_id not found".to_string()));
        }
        let prompt_id = prompt_id.unwrap();
        let mut images: HashMap<String, Vec<u8>> = HashMap::new();

        let (ws_stream, _) = tokio_tungstenite::connect_async(format!(
            "ws://{}/ws?clientId={}",
            self.server_address, self.client_id
        ))
        .await
        .map_err(|e| {
            error!("Failed to connect to WebSocket: {}", e);
            ClientError::from(e)
        })?;
        let (mut _write, mut read) = ws_stream.split();

        while let Some(message) = read.next().await {
            let msg = message.map_err(|e| {
                error!("Failed to read WebSocket message: {}", e);
                ClientError::from(e)
            })?;
            if msg.is_text() {
                let message: HashMap<String, serde_json::Value> =
                    serde_json::from_str(msg.to_text().unwrap()).map_err(|e| {
                        error!("Failed to parse WebSocket message: {}", e);
                        ClientError::from(e)
                    })?;
                if message["type"] == "executing"
                    && message["data"]["node"].is_null()
                    && message["data"]["prompt_id"].as_str().unwrap() == prompt_id
                {
                    break;
                }
            }
        }

        info!("Fetching history and images");

        let history = self.get_history(prompt_id).await?;
        for (_, value) in history[prompt_id]["outputs"].as_object().unwrap() {
            if value["images"].is_array() {
                for image in value["images"].as_array().unwrap() {
                    let filename = image["filename"].as_str().unwrap();
                    let image = self.get_image(filename).await?;
                    if image.is_empty() {
                        continue;
                    }
                    images.insert(filename.to_string(), image);
                }
            }
        }
        Ok(images)
    }

    pub async fn get_history(&self, prompt_id: &str) -> Result<serde_json::Value, ClientError> {
        info!("Getting history for prompt_id: {prompt_id}");
        let client = reqwest::Client::new();
        let res = client
            .get(format!(
                "http://{}/history/{}",
                self.server_address, prompt_id
            ))
            .send()
            .await?;

        // Print .text()
        let res = res.json().await?;
        info!("Got history for prompt_id: {prompt_id}");

        Ok(res)
    }

    pub async fn get_image(&self, filename: &str) -> Result<Vec<u8>, ClientError> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("http://{}/view", self.server_address))
            .query(&[("filename", filename)])
            .send()
            .await?;

        // Print the dimensions of the image
        let bytes = res.bytes().await?;
        info!("Got image: {:?} bytes", bytes.len());
        Ok(bytes.to_vec())
    }

    // pub async fn get_system_stats(&self) -> Result<SystemStats, reqwest::Error> {
    //     let client = reqwest::Client::new();
    //     let res = client
    //         .get(format!("http://{}/system_stats", self.server_address))
    //         .send()
    //         .await?;
    //     let response: SystemStats = res.json().await?;
    //     Ok(response)
    // }
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct SystemStats {
//     pub system: System,
//     pub devices: Vec<Device>,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct System {
//     pub os: String,
//     pub python_version: String,
//     pub embedded_python: bool,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Device {
//     pub name: String,
//     #[serde(rename = "type")]
//     pub device_type: String,
//     pub index: u32,
//     pub vram_total: u64,
//     pub vram_free: u64,
//     pub torch_vram_total: u64,
//     pub torch_vram_free: u64,
// }

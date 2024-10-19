/// Extremely sketchy huggingface client
/// 
use anyhow::Result;
use futures::StreamExt;
use reqwest::{Client as ReqwestClient, Url};
use serde_json::{json, Value};
use tracing::{debug, error};

pub struct HuggingFaceClient {
    client: ReqwestClient,
}

// HF requests require 2 request: one to get the event ID and another to get the response
impl HuggingFaceClient {
    pub fn new() -> Self {
        Self {
            client: ReqwestClient::new(),
        }
    }

    /// Checks if the URL is something like this: <https://xxx.hf.space>
    fn is_hf_space(url: &str) -> bool {
        // Parse the URL using reqwest::Url
        if let Ok(parsed_url) = Url::parse(url) {
            // Check the scheme is https and the domain matches *.hf.space
            if parsed_url.scheme() == "https" {
                if let Some(host) = parsed_url.host_str() {
                    return host.ends_with(".hf.space");
                }
            }
        }
        false
    }

    pub async fn request(&self, url: Url, data: Value) -> Result<Value> {
        if !Self::is_hf_space(url.as_str()) {
            error!("Invalid URL: {:?}", url);
            return Err(anyhow::anyhow!("Invalid URL"));
        }

        // Make the event request
        let event_id = self.event_request(data, &url).await?;

        // Make the invoke request
        let response = self.invoke_request(&url, event_id).await?;

        Ok(response)
    }

    async fn event_request(&self, data: Value, url: &Url) -> Result<String> {
        let url = url.join("/gradio_api/call/predict").unwrap();

        let response = self
            .client
            .post(url)
            .json(&json!({ "data": data }))
            .send()
            .await?;

        // Make sure the status code is 200
        if !response.status().is_success() {
            error!("Status code: {:?}", response.status());
            return Err(anyhow::anyhow!("status code: {}", response.status()));
        }

        let response = response.json::<Value>().await;

        match response {
            Ok(json) => {
                let event_id = json["event_id"].as_str();
                if event_id.is_none() {
                    error!("No event ID found in response: {:?}", json);
                    return Err(anyhow::anyhow!("No event ID found in response"));
                }
                let event_id = event_id.unwrap();
                debug!("Event ID: {event_id}");
                Ok(event_id.to_string())
            }
            Err(e) => {
                error!("Error getting event ID: {:?}", e);
                Err(e.into())
            }
        }
    }

    async fn invoke_request(&self, url: &Url, event_id: String) -> Result<Value> {
        // Build the URL with the event ID
        let url = url
            .join(&format!("/gradio_api/call/predict/{event_id}"))
            .unwrap();

        // Send the GET request for streaming response
        let response = self.client.get(url).send().await?;

        // Ensure the response status code is successful
        if !response.status().is_success() {
            error!("Status code: {:?}", response.status());
            return Err(anyhow::anyhow!("status code: {}", response.status()));
        }

        // Create a stream to read the response in chunks
        let mut stream = response.bytes_stream();

        // We'll collect the streamed data into a buffer
        let mut full_json = json!({});

        // Process each chunk of the stream
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;

            // Convert the chunk to a UTF-8 string
            let chunk_str = String::from_utf8_lossy(&chunk);

            // Print the chunk for debugging
            debug!("Received chunk: {:?}", chunk_str);

            // Handle SSE-style events
            for line in chunk_str.lines() {
                if line.starts_with("data:") {
                    let data = line.trim_start_matches("data: ").trim();
                    debug!("Parsed data: {}", data);

                    // Here, you can deserialize the data if it's JSON or handle it as needed
                    // e.g., for JSON data:
                    if let Ok(parsed_json) = serde_json::from_str::<Value>(data) {
                        debug!("Parsed JSON: {:?}", parsed_json);
                        full_json = parsed_json;
                    }
                }
            }
        }

        Ok(full_json)
    }
}

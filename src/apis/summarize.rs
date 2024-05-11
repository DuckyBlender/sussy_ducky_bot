use log::{error, info};

use crate::{
    apis::openai::{OpenAIRequest, OpenAIRequestMessage},
    structs::{GenerationError, GenerationResponse},
};

// prompt: youtube URL or text
pub async fn summarize(prompt: &str) -> Result<GenerationResponse, GenerationError> {
    info!("Starting summarization response");

    // Check if

    let now = std::time::Instant::now();
    // Send the request to the Perplexity API
    let res = reqwest::Client::new()
        .post("https://api.perplexity.ai/chat/completions")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .bearer_auth(std::env::var("PERPLEXITY_KEY").unwrap_or_default())
        .json(&OpenAIRequest {
            model: "llama-3-sonar-small-32k-online".to_string(),
            messages: vec![
                OpenAIRequestMessage {
                    role: "system".to_string(),
                    content: "Be precise and concise.".to_string(),
                },
                OpenAIRequestMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: 0.2,
            max_tokens: 2048,
        })
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    match res {
        Ok(_) => {
            // info!("Request sent successfully");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
            return Err(GenerationError {
                message: format!("Error sending request: {e}"),
            });
        }
    };

    // Parse the response
    let res = res.unwrap().json::<serde_json::Value>().await;

    // Send the response
    match res {
        Ok(res) => {
            let content = res["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default();
            info!(
                "Generated response using perplexity. Generation took {}s",
                (elapsed * 10.0).round() / 10.0
            );
            Ok(GenerationResponse {
                message: content.to_string(),
            })
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            Err(GenerationError {
                message: format!("Error parsing response: {e}"),
            })
        }
    }
}

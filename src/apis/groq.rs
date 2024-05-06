use log::{error, info};


use super::openai::{OpenAIRequest, OpenAIRequestMessage};

pub enum GroqModels {
    Llama70b,
    Mixtral,
}

impl GroqModels {
    pub fn to_string(&self) -> String {
        match self {
            GroqModels::Llama70b => "llama-70b".to_string(),
            GroqModels::Mixtral => "mixtral".to_string(),
        }
    }
}

pub async fn generate_groq(
    prompt: &String,
    model: GroqModels,
    temperature: f32,
    max_tokens: i32,
) -> Result<String, reqwest::Error> {
    let now = std::time::Instant::now();
    // Send the request to the Perplexity API
    let res = reqwest::Client::new()
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .bearer_auth(std::env::var("GROQ_KEY").unwrap_or_default())
        .json(&OpenAIRequest {
            model: model.to_string(),
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
            temperature,
            max_tokens,
        })
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    // Parse the response
    let res = res.unwrap().json::<serde_json::Value>().await; // TODO: Make an OpenAI response struct
    if let Err(e) = res {
        error!("Error parsing response: {}", e.to_string());
        return Err(e);
    }

    // Send the response
    match res {
        Ok(res) => {
            info!(
                "Generated response for model {} using groq. Generation took {}s",
                model.to_string(), (elapsed * 10.0).round() / 10.0
            );
            let content = res["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default();
            info!(
                "Replying to message using groq. Generation took {}s",
                (elapsed * 10.0).round() / 10.0
            );
            Ok(content.to_string())
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            Err(e)
        }
    }
}

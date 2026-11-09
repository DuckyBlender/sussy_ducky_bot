use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

/// Thin client for the OpenAI-compatible llama.cpp server.
pub struct Llm {
    http: reqwest::Client,
    base_url: String,
    model: String,
}

#[derive(Deserialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

/// One message of the conversation sent to the model.
pub struct ChatMessage {
    pub role: &'static str,
    pub text: String,
    /// Images as `data:` URLs.
    pub images: Vec<String>,
}

impl Llm {
    pub fn new(base_url: &str, model: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build HTTP client");
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_owned(),
            model,
        }
    }

    /// Whether the loaded model accepts images (llama-server started with an mmproj).
    /// Asked every time so loading a projector later needs no bot restart.
    pub async fn supports_vision(&self) -> bool {
        let props: Result<Value> = async {
            Ok(self
                .http
                .get(format!("{}/props", self.base_url))
                .timeout(Duration::from_secs(5))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?)
        }
        .await;
        match props {
            Ok(p) => p["modalities"]["vision"].as_bool().unwrap_or(false),
            Err(e) => {
                log::warn!("could not read /props, assuming no vision: {e}");
                false
            }
        }
    }

    pub async fn chat(&self, system: &str, history: &[ChatMessage]) -> Result<String> {
        let mut messages = vec![json!({ "role": "system", "content": system })];
        for m in history {
            let content = if m.images.is_empty() {
                json!(m.text)
            } else {
                let mut parts: Vec<Value> = m
                    .images
                    .iter()
                    .map(|url| json!({ "type": "image_url", "image_url": { "url": url } }))
                    .collect();
                if !m.text.is_empty() {
                    parts.push(json!({ "type": "text", "text": m.text }));
                }
                json!(parts)
            };
            messages.push(json!({ "role": m.role, "content": content }));
        }

        let res = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&json!({ "model": self.model, "messages": messages, "max_tokens": 2048 }))
            .send()
            .await
            .context("LLM request failed")?;
        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            bail!(
                "LLM returned {status}: {}",
                body.chars().take(300).collect::<String>()
            );
        }

        let resp: CompletionResponse = res.json().await.context("bad LLM response")?;
        let text = resp
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default()
            .trim()
            .to_owned();
        if text.is_empty() {
            bail!("LLM returned an empty reply");
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a running llama-server: `LLM_URL=http://host:8081 cargo test -- --ignored`
    #[tokio::test]
    #[ignore = "needs a running llama-server"]
    async fn talks_to_server() {
        let url = std::env::var("LLM_URL").unwrap_or_else(|_| "http://localhost:8081".into());
        let llm = Llm::new(&url, "bolt".into());
        println!("vision: {}", llm.supports_vision().await);
        let reply = llm
            .chat(
                "Answer in one word.",
                &[ChatMessage {
                    role: "user",
                    text: "What colour is the sky on a clear day?".into(),
                    images: vec![],
                }],
            )
            .await
            .unwrap();
        println!("reply: {reply}"); // chat() already errors on an empty reply
    }
}

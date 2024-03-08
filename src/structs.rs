use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct PerplexityRequest {
    pub model: String,
    pub messages: Vec<PerplexityRequestMessage>,
}

#[derive(Debug, Serialize)]
pub struct PerplexityRequestMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct OllamaChatRequest {
    pub model: String,
    pub messages: Vec<OllamaChatRequestMessage>,
    pub stream: bool,
}

#[derive(Debug, Serialize)]
pub struct OllamaChatRequestMessage {
    pub role: String, // user | assistant
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaChatResponse {
    pub model: String,
    pub created_at: String,
    pub message: OllamaChatResponseMessage,
    pub done: bool,
    pub total_duration: i64,
    pub load_duration: i64,
    pub prompt_eval_count: Option<i32>,
    pub prompt_eval_duration: i64,
    pub eval_count: i32,
    pub eval_duration: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaChatResponseMessage {
    pub role: String,
    pub content: String,
}

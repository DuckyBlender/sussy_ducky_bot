use serde::Serialize;

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
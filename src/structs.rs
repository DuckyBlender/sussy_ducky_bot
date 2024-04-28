use serde::Serialize;

pub const INTERVAL_SEC: u64 = 5;

#[derive(Debug, Serialize)]
pub struct PerplexityRequest {
    pub model: String,
    pub messages: Vec<PerplexityRequestMessage>,
    pub temperature: f32,
}

#[derive(Debug, Serialize)]
pub struct PerplexityRequestMessage {
    pub role: String,
    pub content: String,
}

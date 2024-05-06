use serde::Serialize;

#[derive(Serialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIRequestMessage>,
    pub temperature: f32,
    pub max_tokens: i32,
}

#[derive(Serialize)]
pub struct OpenAIRequestMessage {
    pub role: String,
    pub content: String,
}
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

#[derive(Serialize, Deserialize, Debug)]
pub struct GPT4ImageUrl {
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GPT4Content {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub image_url: Option<GPT4ImageUrl>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GPT4Message {
    pub role: String,
    pub content: Vec<GPT4Content>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GPT4Request {
    pub model: String,
    pub messages: Vec<GPT4Message>,
    pub max_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DallERequest {
    pub model: String,
    pub prompt: String,
    pub size: String, // there is probably a better way to do this but it's good enough for now
}

// GPT Response
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    pub logprobs: Option<String>,
    pub finish_reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GPT4Response {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<i64>,
    pub model: String,
    pub system_fingerprint: String,
    pub choices: Vec<Choice>,
    pub usage: TokenUsage,
}

// DALL-E Response
#[derive(Serialize, Deserialize, Debug)]
pub struct Image {
    pub url: String,
    pub revised_prompt: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DallEResponse {
    pub created: i64,
    pub data: Vec<Image>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PerplexityResponse {
    pub id: String,
    pub model: String,
    pub created: i64,
    pub usage: TokenUsage,
    pub object: String,
    pub choices: Vec<Choice>,
}

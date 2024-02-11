use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TTSRequest {
    pub model: String,
    pub input: String,
    pub voice: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    pub context: Option<Vec<i32>>, // optional if raw is true
    pub total_duration: i64,
    pub load_duration: i64,
    pub prompt_eval_count: Option<i32>,
    pub prompt_eval_duration: i64,
    pub eval_count: i32,
    pub eval_duration: i64,
}

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

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockRequest {
    pub taskType: String,
    pub textToImageParams: BedrockTextToImageParams,
    pub imageGenerationConfig: BedrockImageGenerationConfig,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockTextToImageParams {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negativeText: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockImageGenerationConfig {
    pub numberOfImages: i32,
    pub quality: String, // standard | premium
    pub height: i32,
    pub width: i32,
    pub cfgScale: f32,
    pub seed: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BedrockResponse {
    pub images: Vec<String>,
    pub error: Option<String>,
}

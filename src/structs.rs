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

#[derive(Serialize, Deserialize)]
pub struct GPT4ImageUrl {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct GPT4Content {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub image_url: Option<GPT4ImageUrl>,
}

#[derive(Serialize, Deserialize)]
pub struct GPT4Message {
    pub role: String,
    pub content: Vec<GPT4Content>,
}

#[derive(Serialize, Deserialize)]
pub struct GPT4Request {
    pub model: String,
    pub messages: Vec<GPT4Message>,
    pub max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
pub struct DallERequest {
    pub model: String,
    pub prompt: String,
    pub size: String, // there is probably a better way to do this but it's good enough for now
}

// GPT Response
#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    pub logprobs: Option<String>,
    pub finish_reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Serialize, Deserialize)]
pub struct GPT4Response {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub system_fingerprint: String,
    pub choices: Vec<Choice>,
    pub usage: TokenUsage,
}

// DALL-E Response
#[derive(Serialize, Deserialize)]
pub struct Image {
    pub url: String,
    pub revised_prompt: String,
}

#[derive(Serialize, Deserialize)]
pub struct DallEResponse {
    pub created: i64,
    pub data: Vec<Image>,
}

// {
//   "id": "936518f4-f215-4ba5-9653-7766c0c1a1b9",
//   "model": "sonar-small-online",
//   "created": 11284578,
//   "usage": {
//     "prompt_tokens": 44,
//     "completion_tokens": 21,
//     "total_tokens": 65
//   },
//   "object": "chat.completion",
//   "choices": [
//     {
//       "index": 0,
//       "finish_reason": "stop",
//       "message": {
//         "role": "assistant",
//         "content": "The Milky Way galaxy contains between 100 and 400 billion stars."
//       },
//       "delta": {
//         "role": "assistant",
//         "content": ""
//       }
//     }
//   ]
// }

#[derive(Serialize, Deserialize)]
pub struct PerplexityResponse {
    pub id: String,
    pub model: String,
    pub created: i64,
    pub usage: TokenUsage,
    pub object: String,
    pub choices: Vec<Choice>,
}

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

// {
//   "id": "25ffbb87-afe8-4ec6-80ba-89fbebd8f43d",
//   "model": "mistral-7b-instruct",
//   "created": 3525046,
//   "usage": {
//     "prompt_tokens": 25,
//     "completion_tokens": 92,
//     "total_tokens": 117
//   },
//   "object": "chat.completion",
//   "choices": [
//     {
//       "index": 0,
//       "finish_reason": "stop",
//       "message": {
//         "role": "assistant",
//         "content": "The number of stars in our galaxy, the Milky Way, is estimated to be between 100-400 billion. However, the most commonly accepted estimate is around 100-120 billion stars. Keep in mind that this is a rough estimate as the actual number could be higher or lower. Recent studies suggest that there could be as many as 1-3 trillion stars. (Source: European Space Agency)"
//       },
//       "delta": {
//         "role": "assistant",
//         "content": ""
//       }
//     }
//   ]
// }

#[derive(Debug, Deserialize)]
pub struct PerplexityResponse {
    pub id: String,
    pub model: String,
    pub created: i64,
    pub usage: PerplexityResponseUsage,
    pub object: String,
    pub choices: Vec<PerplexityResponseChoice>,
}

#[derive(Debug, Deserialize)]
pub struct PerplexityResponseUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Deserialize)]
pub struct PerplexityResponseChoice {
    pub index: i32,
    pub finish_reason: String,
    pub message: PerplexityResponseMessage,
    pub delta: PerplexityResponseMessage,
}

#[derive(Debug, Deserialize)]
pub struct PerplexityResponseMessage {
    pub role: String,
    pub content: String,
}

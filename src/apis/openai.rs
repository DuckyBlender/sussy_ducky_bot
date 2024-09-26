use std::{env, str::FromStr};

use anyhow::Result;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client as ReqwestClient,
};
use serde_json::{json, Value};
use tracing::{debug, error};

use crate::BotCommand;

const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

const OPENROUTER_HEADERS: [&str; 2] = [
    "X-Title: sussy_ducky_bot",
    "HTTP-Referer: https://t.me/sussy_ducky_bot",
];

#[derive(Debug)]
enum Providers {
    OpenRouter,
}

pub struct OpenAIClient {
    client: ReqwestClient,
}

impl OpenAIClient {
    pub fn new() -> Self {
        Self {
            client: ReqwestClient::new(),
        }
    }

    fn get_model_and_provider(model: &BotCommand, image: bool) -> (String, Providers) {
        match model {
            BotCommand::Llama => {
                if image {
                    (
                        "meta-llama/llama-3.2-11b-vision-instruct:free".to_string(),
                        Providers::OpenRouter,
                    )
                } else {
                    (
                        "meta-llama/llama-3.1-8b-instruct:free".to_string(),
                        Providers::OpenRouter,
                    )
                }
            }
            BotCommand::Lobotomy => (
                "meta-llama/llama-3.2-1b-instruct".to_string(),
                Providers::OpenRouter,
            ),
            BotCommand::Caveman => (
                "meta-llama/llama-3.1-8b-instruct:free".to_string(),
                Providers::OpenRouter,
            ),
            BotCommand::Help | BotCommand::Start | BotCommand::Flux => unreachable!(),
        }
    }

    fn get_provider_base_url(provider: &Providers) -> &'static str {
        match provider {
            Providers::OpenRouter => OPENROUTER_BASE_URL,
        }
    }

    fn get_api_key(provider: &Providers) -> String {
        match provider {
            Providers::OpenRouter => env::var("OPENROUTER_KEY").expect("OPENROUTER_KEY is not set"),
        }
    }

    fn get_system_prompt(model: &BotCommand) -> Option<&'static str> {
        match model {
            BotCommand::Caveman => Some("You are a caveman. Speak like a caveman would. All caps, simple words, grammar mistakes etc."),
            BotCommand::Llama => Some("Be concise and precise. Don't be verbose. Answer in the user's language."),
            BotCommand::Lobotomy => None,
            BotCommand::Help | BotCommand::Start | BotCommand::Flux => unreachable!(),
        }
    }

    fn get_additional_headers(provider: &Providers) -> HeaderMap {
        match provider {
            Providers::OpenRouter => {
                let mut headers = HeaderMap::new();
                for header in &OPENROUTER_HEADERS {
                    let header_parts: Vec<&str> = header.splitn(2, ": ").collect();
                    let header_name = HeaderName::from_str(header_parts[0].trim()).unwrap();
                    let header_value = HeaderValue::from_str(header_parts[1].trim()).unwrap();
                    headers.insert(header_name, header_value);
                }
                headers
            }
        }
    }

    pub async fn openai_request(
        &self,
        prompt: &str,
        base64_img: Option<&str>,
        model: BotCommand,
    ) -> Result<String> {
        let (model_str, provider) = Self::get_model_and_provider(&model, base64_img.is_some());
        let provider_base_url = Self::get_provider_base_url(&provider);
        let api_key = Self::get_api_key(&provider);
        let system_prompt = Self::get_system_prompt(&model);

        let mut messages = vec![];

        // Add system message if provided
        if let Some(system) = system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        // Construct user message content
        let mut user_content = vec![json!({
            "type": "text",
            "text": prompt
        })];

        // Add image if provided
        if let Some(image) = base64_img {
            user_content.push(json!({
                "type": "image_url",
                "image_url": {
                    "url": format!("data:image/jpeg;base64,{}", image)
                }
            }));
        }

        // Add user message
        messages.push(json!({
            "role": "user",
            "content": user_content
        }));

        let additional_headers = Self::get_additional_headers(&provider);

        debug!("headers: {:?}", additional_headers);

        let json_request = &json!({
            "model": model_str,
            "messages": messages,
            "max_tokens": 512,
        });

        let response = self
            .client
            .post(format!("{provider_base_url}/chat/completions"))
            .bearer_auth(api_key)
            .headers(additional_headers)
            .json(json_request)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            if status.as_u16() == 429 {
                // too many requests
                let message: Value = response.json().await?;
                let message = message["message"].as_str().unwrap_or("Rate limited");
                error!("Rate limited: {}", message);
                return Err(anyhow::anyhow!("rate limited: {}", message));
            }

            let response_body: Value = response.json().await?;
            let response_body_pretty = serde_json::to_string_pretty(&response_body)?;
            error!("Status non-200: {}", response_body_pretty);
            return Err(anyhow::anyhow!("something went wrong :("));
        }

        let response_body = response.text().await?;
        let json_response: Value = serde_json::from_str(&response_body)?;
        let text_response = json_response["choices"][0]["message"]["content"]
            .as_str();

        if text_response.is_none() {
            error!("no text found in the response: {:?}", json_response);
            return Err(anyhow::anyhow!("no text found in the response"));
        }
        let text_response = text_response.unwrap();
        Ok(text_response.to_string())
    }
}

use std::{env, str::FromStr};

use anyhow::Result;
use reqwest::Client as ReqwestClient;
use serde_json::{json, Value};
use tracing::{debug, error};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::BotCommand;

// this file is actually for groq and openrouter, they use the openai standard

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

const OPENROUTER_HEADERS: [&str; 2] = [
    "X-Title: sussy_ducky_bot",
    "HTTP-Referer: https://t.me/sussy_ducky_bot",
];

#[derive(Debug)]
enum Providers {
    Groq,
    OpenRouter,
}

// model list is in BotCommand

pub async fn openai_request(
    client: &ReqwestClient,
    prompt: &str,
    base64_img: Option<&str>,
    model: BotCommand,
) -> Result<String> {
    let model_str = match model {
        BotCommand::Caveman => "llama-3.1-70b-versatile",
        BotCommand::Llama => "llama-3.1-70b-versatile",
        BotCommand::Pixtral => "mistralai/pixtral-12b:free",
        BotCommand::Vision => "qwen/qwen-2-vl-7b-instruct:free",
        BotCommand::Help | BotCommand::Start => unreachable!(),
    };

    let provider = match model {
        BotCommand::Caveman | BotCommand::Llama => Providers::Groq,
        BotCommand::Pixtral | BotCommand::Vision => Providers::OpenRouter,
        BotCommand::Help | BotCommand::Start => unreachable!(),
    };

    let provider_base_url = match provider {
        Providers::Groq => GROQ_BASE_URL,
        Providers::OpenRouter => OPENROUTER_BASE_URL,
    };

    let api_key = match provider {
        Providers::Groq => env::var("GROQ_KEY").expect("GROQ_KEY is not set"),
        Providers::OpenRouter => env::var("OPENROUTER_KEY").expect("OPENROUTER_KEY is not set"),
    };

    let system_prompt = match model {
        BotCommand::Caveman => Some("You are a caveman. Speak like a caveman would. All caps, simple words, grammar mistakes etc."),
        BotCommand::Llama => Some("Be concise and precise. Don't be verbose. Answer in the user's language."),
        BotCommand::Pixtral | BotCommand::Vision => None,
        BotCommand::Help | BotCommand::Start => unreachable!(),
    };

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

    let additional_headers = match provider {
        Providers::Groq => HeaderMap::new(),
        Providers::OpenRouter => {
            let mut headers = HeaderMap::new();
            for header in OPENROUTER_HEADERS.iter() {
                let header_parts: Vec<&str> = header.splitn(2, ": ").collect();
                let header_name = HeaderName::from_str(header_parts[0].trim()).unwrap();
                let header_value = HeaderValue::from_str(header_parts[1].trim()).unwrap();
                headers.insert(header_name, header_value);
            }
            headers
        }
    };

    debug!("headers: {:?}", additional_headers);

    let json = &json!({
        "model": model_str,
        "messages": messages,
        "max_tokens": 512,
    });

    // info!("JSON: {}", serde_json::to_string_pretty(json)?);

    let response = client
        .post(format!("{provider_base_url}/chat/completions"))
        .bearer_auth(api_key)
        .headers(additional_headers)
        .json(json)
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        if status.as_u16() == 429 {
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
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No text found in the response"))?;
    Ok(text_response.to_string())
}

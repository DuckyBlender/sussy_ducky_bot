use std::env;

use anyhow::Result;
use reqwest::Client as ReqwestClient;
use serde_json::{json, Value};
use tracing::error;

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
        BotCommand::Llava => "llava-v1.5-7b-4096-preview",
        BotCommand::Pixtral => "mistralai/pixtral-12b:free",
        BotCommand::Qwen => "qwen/qwen-2-vl-7b-instruct:free",
        BotCommand::Help | BotCommand::Start => unreachable!(),
    };

    let provider = match model {
        BotCommand::Caveman | BotCommand::Llama | BotCommand::Llava => Providers::Groq,
        BotCommand::Pixtral | BotCommand::Qwen => Providers::OpenRouter,
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
        BotCommand::Llava | BotCommand::Pixtral | BotCommand::Qwen => None,
        BotCommand::Help | BotCommand::Start => unreachable!(),
    };

    let mut request_body = json!({
        "model": model_str,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]
            }
        ],
        "max_tokens": 300
    });

    // Add image to the request
    if let Some(img) = base64_img {
        let img_url = format!("data:image/jpeg;base64,{}", img);
        let img_json = json!({
            "type": "image_url",
            "image_url": {
                "url": img_url
            }
        });
        let messages = request_body["messages"].as_array_mut().unwrap();
        let user = messages[0].as_object_mut().unwrap();
        let content = user["content"].as_array_mut().unwrap();
        content.push(img_json);
    }

    // Add system prompt
    if let Some(prompt) = system_prompt {
        let system_prompt = json!({
            "role": "system",
            "content": [
                {
                    "type": "text",
                    "text": prompt
                }
            ]
        });
        let messages = request_body["messages"].as_array_mut().unwrap();
        messages.push(system_prompt);
    }

    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    let additional_headers = match provider {
        Providers::Groq => HeaderMap::new(),
        Providers::OpenRouter => {
            let mut headers = HeaderMap::new();
            for header in OPENROUTER_HEADERS.iter() {
                let parts: Vec<&str> = header.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    headers.insert(
                        HeaderName::from_lowercase(parts[0].as_bytes()).unwrap(),
                        HeaderValue::from_str(parts[1]).unwrap(),
                    );
                }
            }
            headers
        }
    };

    let response = client
        .post(format!("{provider_base_url}/chat/completions"))
        .bearer_auth(api_key)
        .headers(additional_headers)
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        response.json().await?;
        let response_body = serde_json::to_string_pretty(&())?;
        error!("Status non-200: {}", response_body);
        return Err(anyhow::anyhow!("something went wrong :("));
    }

    let response_body = response.text().await?;
    let json_response: Value = serde_json::from_str(&response_body)?;
    let text_resposne = json_response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No text found in the response"))?;
    Ok(text_resposne.to_string())
}

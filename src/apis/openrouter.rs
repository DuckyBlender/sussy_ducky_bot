use anyhow::Result;
use reqwest::Client as ReqwestClient;
use serde_json::Value;

const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

pub async fn send_openrouter_request(
    client: &ReqwestClient,
    api_key: &str,
    prompt: &str,
    system_prompt: &str,
    base64_img: Option<&str>,
) -> Result<Value> {
    let model = if base64_img.is_some() {
        "qwen/qwen-2-vl-7b-instruct:free"
        // mistralai/pixtral-12b:free
    } else {
        return Err(anyhow::anyhow!("For now, image is required for OpenRouter"));
    };

    let mut messages = vec![
        {
            serde_json::json!({
                "role": "system",
                "content": system_prompt
            })
        },
        {
            serde_json::json!({
                "role": "user",
                "content": prompt
            })
        },
    ];

    if let Some(img) = base64_img {
        messages.push(serde_json::json!({
            "type": "image_url",
            "image_url": format!("data:image/jpeg;base64,{}", img)
        }));
    }

    let request_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "max_tokens": 1024
    });

    let response = client
        .post(format!("{}/chat/completions", "https://api.openrouter.ai"))
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await?;

    let response_body = response.text().await?;
    let json_response: Value = serde_json::from_str(&response_body)?;
    Ok(json_response)
}

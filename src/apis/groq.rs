use anyhow::Result;
use reqwest::Client as ReqwestClient;
use serde_json::Value;

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

pub async fn send_groq_request(
    client: &ReqwestClient,
    api_key: &str,
    prompt: &str,
    system_prompt: &str,
    base64_img: Option<&str>,
) -> Result<Value> {
    let model = match base64_img {
        Some(_) => "llava-v1.5-7b-4096-preview",
        None => "llama-3.1-70b-versatile",
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
        .post(format!("{GROQ_BASE_URL}/chat/completions"))
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await?;

    let response_body = response.text().await?;
    let json_response: Value = serde_json::from_str(&response_body)?;
    Ok(json_response)
}

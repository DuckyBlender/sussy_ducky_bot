use aws_config::BehaviorVersion;
// File for AWS Bedrock commands
use crate::structs::GenerationError;
use crate::structs::*;
use aws_sdk_bedrockruntime::primitives::Blob;
use log::error;
use log::info;
use serde_json::json;
use std::str;

pub enum ClaudeModels {
    Haiku,
    Sonnet,
    Opus,
}

impl std::fmt::Display for ClaudeModels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Haiku => write!(f, "anthropic.claude-3-haiku-20240307-v1:0"),
            Self::Sonnet => write!(f, "anthropic.claude-3-sonnet-20240229-v1:0"),
            Self::Opus => write!(f, "anthropic.claude-3-opus-20240229-v1:0"),
        }
    }
}

pub async fn claude_generation(
    prompt: Option<&str>, // prompt is optional only when image is provided
    image: Option<&str>,  // base64 image
    model: ClaudeModels,
) -> Result<GenerationResponse, GenerationError> {
    info!("Starting AWS bedrock generation");

    // Check if the prompt AND image are both empty
    if prompt.is_none() && image.is_none() {
        return Err(GenerationError {
            message: "Prompt and image cannot both be empty".to_string(),
        });
    }

    // Create the AWS Bedrock client
    let now = std::time::Instant::now();

    // Send the request to the AWS Bedrock API
    let mut json_body = json!(
            {
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 2048,
        "messages": [
            {
                "role": "user",
                "content": [
                    // {
                    //     "type": "image",
                    //     "source": {
                    //         "type": "base64",
                    //         "media_type": "image/jpeg",
                    //         "data": "iVBORw..."
                    //     }
                    // },
                    {
                        "type": "text",
                        "text": prompt.unwrap_or("What is in this image?")
                    }
                ]
            }
        ]
    }
        );

    // Add the image to the JSON body if it exists
    if let Some(image) = image {
        json_body["messages"][0]["content"] = json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/jpeg",
                "data": image
            }
        });
    }

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("us-west-2") // this has the most models, but literally the highest latency
        .load()
        .await;

    let aws_client = aws_sdk_bedrockruntime::Client::new(&aws_config);
    let result = aws_client
        .invoke_model()
        .model_id(model.to_string())
        .content_type("application/json")
        .body(Blob::new(serde_json::to_string(&json_body).unwrap()))
        .send()
        .await;

    // Check for what reason the result is blocked
    if let Err(e) = result {
        // Return the response
        let err = e.into_service_error();
        error!("Error: {:?}", err);
        return Err(GenerationError {
            message: format!("Error: {err:?}"),
        });
    }
    let result = result.unwrap();

    // Convert the blob to a JSON
    let output_str = str::from_utf8(result.body().as_ref()).unwrap();
    let output_json: serde_json::Value = serde_json::from_str(output_str).unwrap();

    // Get the message from the JSON
    // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-anthropic-claude-messages.html
    let message = output_json["content"][0]["text"].as_str().unwrap();

    // Calculate the time taken
    let elapsed = now.elapsed().as_secs_f32();

    info!(
        "Generated response using bedrock. Generation took {}s",
        (elapsed * 10.0).round() / 10.0
    );

    // Return the response
    Ok(GenerationResponse {
        message: message.to_string(),
    })
}

// File for AWS Bedrock commands
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::Engine as _;
use log::info;
use serde_json::json;
use teloxide::net::Download;
use teloxide::payloads::SendMessageSetters;
use teloxide::payloads::SendPhotoSetters;
use teloxide::types::UserId;
use teloxide::{requests::Requester, types::Message, Bot, RequestError};

use crate::utils::ModelType;
use aws_sdk_bedrockruntime::primitives::Blob;
use std::str;

pub async fn bedrock(
    bot: Bot,
    msg: Message,
    prompt: Option<String>,
    model: ModelType,
    aws_client: aws_sdk_bedrockruntime::Client,
) -> Result<(), RequestError> {
    info!("Starting AWS bedrock function");

    // Check if the user is from the owner
    if msg.from().unwrap().id != UserId(std::env::var("OWNER_ID").unwrap().parse().unwrap()) {
        bot.send_message(
            msg.chat.id,
            "You are not the owner. Please mention @DuckyBlender if you want to use this command!",
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    // Check if the model is one of bedrocks models
    let bedrock_models = ModelType::return_bedrock();
    if !bedrock_models.contains(&model) {
        bot.send_message(msg.chat.id, "Error: Invalid model")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }

    let prompt = match prompt {
        Some(prompt) => Some(prompt),
        None => {
            // Image Variation has an optional prompt
            if model != ModelType::AmazonTitanImageVariation {
                
            let bot_msg = bot
                .send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;

            // Wait 5 seconds
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // Deleting the messages
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
            return Ok(());
            } else {
                None
            }
        }
    };

    let mut img = String::new();
    if model == ModelType::AmazonTitanImageVariation {
        // Check if there is an image or sticker attached in the reply
        let img_attachment = if let Some(reply) = msg.reply_to_message() {
            reply
                .photo()
                .map(|photo| photo.last().unwrap().file.id.clone())
                .or_else(|| reply.sticker().map(|sticker| &sticker.file.id).cloned())
        } else {
            let bot_msg = bot
                .send_message(msg.chat.id, "No image or sticker provided")
                .reply_to_message_id(msg.id)
                .await?;

            // Wait 5 seconds and delete the users and the bot's message
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // Deleting the messages
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;

            return Ok(());
        };

        if img_attachment.is_none() {
            let bot_msg = bot
                .send_message(msg.chat.id, "No image or sticker provided")
                .reply_to_message_id(msg.id)
                .await?;

            // Wait 5 seconds and delete the users and the bot's message
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // Deleting the messages
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(bot_msg.chat.id, bot_msg.id).await?;
            return Ok(());
        }

        // Download the image
        let img_attachment = img_attachment.unwrap();
        let img_file = bot.get_file(&img_attachment).await.unwrap();
        let img_url = img_file.path;
        let mut buf: Vec<u8> = Vec::new();
        bot.download_file(&img_url, &mut buf).await.unwrap();
        img = BASE64.encode(&buf);

    }

    // Send a message to the chat to show that the bot is generating a response
    let generating_message = bot
        .send_message(msg.chat.id, "Generating response...")
        .reply_to_message_id(msg.id)
        .disable_notification(true)
        .await?;

    // Send "typing indicator" to show that the bot is typing
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    let now = std::time::Instant::now();

    // Send the request to the AWS Bedrock API
    let json_body = match model {
        // amazon text models
        ModelType::AmazonTitanText | ModelType::AmazonTitanTextLite => {
            json!(
                {
                    "inputText": prompt,
                    // Other default parameters:
                    // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-text.html
                }
            )
        }

        // cohere commandR and commandR+ models
        ModelType::CommandR | ModelType::CommandRPlus => {
            json!({
                "message": prompt,
                // Other default parameters:
                // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-cohere-command-r-plus.html

            })
        }

        ModelType::AmazonTitanImage => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-image.html
            let mut json = json!({
                "taskType": "TEXT_IMAGE",
                "textToImageParams": {
                    "text": prompt,
                    // "negativeText": ""
                },
                "imageGenerationConfig": {
                    "numberOfImages": 1,
                    "height": 512,
                    "width": 512,
                    // rest of the default parameters in the URL above
                }
            });

            // if the "text" field is empty, remove it
            if json["textToImageParams"]["text"].as_str().unwrap().is_empty() {
                json["textToImageParams"].as_object_mut().unwrap().remove("text");
            }

            json
        }

        ModelType::AmazonTitanImageVariation => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-image.html
            let mut json = json!({
                 "taskType": "IMAGE_VARIATION",
                 "imageVariationParams": {
                     "text": prompt,
                    //  "negativeText": "", 
                     "images": [img],
                     "similarityStrength": 0.7, // default
                 },
                 
            });
            // if the "text" field is empty, remove it
            if json["imageVariationParams"]["text"].as_str().unwrap().is_empty() {
                json["imageVariationParams"].as_object_mut().unwrap().remove("text");
            }
            json
        }
        _ => {
            unreachable!();
        }
    };

    let result = aws_client
        .invoke_model()
        .model_id(model.to_string())
        .content_type("application/json")
        .body(Blob::new(serde_json::to_string(&json_body).unwrap()))
        .send()
        .await
        .unwrap();

    // Convert the blob to a JSON
    let output = str::from_utf8(result.body().as_ref()).unwrap();
    let output_json: serde_json::Value = serde_json::from_str(output).unwrap();
    let output_txt = match model {
        ModelType::AmazonTitanText | ModelType::AmazonTitanTextLite => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-text.html
            output_json["results"][0]["outputText"].as_str().unwrap()
        }
        ModelType::CommandR | ModelType::CommandRPlus => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-cohere-command-r-plus.html
            output_json["text"].as_str().unwrap()
        }
        ModelType::AmazonTitanImage | ModelType::AmazonTitanImageVariation => {
            // https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-image.html
            output_json["images"][0].as_str().unwrap()
        }
        _ => {
            unreachable!();
        }
    };

    // Calculate the time taken
    let elapsed = now.elapsed().as_secs_f32();

    info!(
        "Replying to message using bedrock. Generation took {}s",
        (elapsed * 10.0).round() / 10.0
    );

    // Send the message
    match model {
        ModelType::AmazonTitanImage => {
            // Convert the output_txt (which is base64) into an InputFile
            let output_txt = BASE64.decode(output_txt).unwrap();
            let output_txt = teloxide::types::InputFile::memory(output_txt);
            bot.send_photo(msg.chat.id, output_txt)
                .caption(prompt.unwrap_or_default()) // blank prompt if it doesn't exist
                .await?;
            bot.delete_message(generating_message.chat.id, generating_message.id)
                .await?;
        }
        _ => {
            bot.edit_message_text(msg.chat.id, generating_message.id, output_txt)
                .await?;
        }
    };

    Ok(())
}

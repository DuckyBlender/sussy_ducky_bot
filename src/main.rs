use base64::prelude::*;
use log::{debug, info};
use serde_json::Value;
use teloxide::{net::Download, prelude::*, RequestError};

mod ollama;
use ollama::*;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    teloxide::repl(bot, handler).await;
}

fn parse_command(msg: &Message) -> (Option<&str>, Option<&str>) {
    let text = msg.text().unwrap_or("");
    let mut iter = text.splitn(2, ' ');
    let command = iter.next();
    // Check if the @botname exists right after the command
    // If it is, check if the bot is mentioned
    // If it is, remove the @botname
    let command = command.map(|c| {
        if c.starts_with("@") {
            let bot_name = std::env::var("BOT_NAME").unwrap_or("sussy_ducky_bot".to_string());
            if c == format!("@{}", bot_name) {
                c.split('@').next().unwrap()
            } else {
                c
            }
        } else {
            c
        }
    });

    (command, iter.next())
}

fn parse_command_in_caption(msg: &Message) -> (Option<&str>, Option<&str>) {
    let text = msg.caption().unwrap_or("");
    let mut iter = text.splitn(2, ' ');
    let command = iter.next();
    // Check if the @botname exists right after the command
    // If it is, check if the bot is mentioned
    // If it is, remove the @botname
    let command = command.map(|c| {
        if c.starts_with("@") {
            // get the bot name from env 
            let bot_name = std::env::var("BOT_NAME").unwrap_or("sussy_ducky_bot".to_string());
            if c == format!("@{}", bot_name) {
                c.split('@').next().unwrap()
            } else {
                c
            }
        } else {
            c
        }
    });

    (command, iter.next())
}

async fn handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    // info!("Received message: {}", msg.text().unwrap_or(""));

    // Check if the message is a message or an image with a caption
    if msg.photo().is_some() && msg.caption().is_some() {
        info!("Message is an image with a caption");
        let (command, args) = parse_command_in_caption(&msg);
        debug!("Command: {:?}, args: {:?}", command, args);
        match command {
            Some("/llava") | Some("/l") => {
                let prompt = args.unwrap_or("").to_string();
                debug!("Executing llava command with prompt: {}", prompt);
                llava(bot, msg, prompt).await?;
            }
            _ => {}
        }
    } else if msg.text().is_some() {
        info!("Message is a text message");
        let (command, args) = parse_command(&msg);
        match command {
            Some("/mistral") | Some("/m") => {
                let prompt = args.unwrap_or("").to_string();
                debug!("Executing mistral command with prompt: {}", prompt);
                mistral(bot, msg, prompt).await?;
            }
            Some("/llava") | Some("/l") => {
                let prompt = args.unwrap_or("").to_string();
                debug!("Executing llava reply command with prompt: {}", prompt);
                llava(bot, msg, prompt).await?;
            }
            Some("/help") | Some("/h") => {
                bot.send_message(msg.chat.id, "Available commands:\n/mistral or /m: generate text\n/llava or /l: generate text from image")
                    .reply_to_message_id(msg.id)
                    .await?;
            }
            Some("/start") => {
                bot.send_message(msg.chat.id, "Welcome to Sussy Ducky Bot (because all the good names were taken)\nAvailable commands:\n/mistral or /m: generate text\n/llava or /l: generate text from image")
                    .reply_to_message_id(msg.id)
                    .await?;
            }
            _ => {}
        }
    } else {
        info!("Message is not a text message nor an image with a caption");
    }

    Ok(())
}

async fn mistral(bot: Bot, msg: Message, prompt: String) -> Result<Message, RequestError> {
    info!("Starting mistral function");
    // If the prompt is empty, check if there is a reply
    let prompt = if prompt.is_empty() {
        if let Some(reply) = msg.reply_to_message() {
            reply.text().unwrap_or("").to_string()
        } else {
            bot.send_message(msg.chat.id, "No prompt provided")
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    } else {
        prompt
    };

    // Check if prompt is nothing
    if prompt.is_empty() {
        bot.send_message(msg.chat.id, "No prompt provided")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(msg);
    }

    // Send typing action
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    // Send the request
    let res = reqwest::Client::new()
        .post("http://localhost:11434/api/generate")
        .json(&OllamaRequest {
            model: "mistral".to_string(),
            prompt,
            stream: false,
            images: None,
        })
        .send()
        .await;

    match res {
        Ok(_) => {
            log::info!("Request sent successfully");
        }
        Err(e) => {
            log::debug!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {}", e))
                .reply_to_message_id(msg.id)
                .await?;
            return Ok(msg);
        }
    };

    // Parse the response
    let res = res.unwrap().json::<OllamaResponse>().await;

    // Send the response
    match res {
        Ok(res) => {
            bot.send_message(msg.chat.id, res.response)
                .reply_to_message_id(msg.id)
                .await
        }
        Err(e) => {
            log::debug!("Error parsing response: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {}", e))
                .reply_to_message_id(msg.id)
                .await
        }
    }
}

async fn llava(bot: Bot, msg: Message, mut prompt: String) -> Result<Message, RequestError> {
    log::info!("Starting llava function");

    // log::info!("Prompt: {}", prompt);

    if prompt.is_empty() {
        prompt = "What is in this image?".to_string();
    }

    let photo = match msg.photo() {
        Some(photos) => photos.last().unwrap(),
        None => {
            // Check if there is a reply
            if let Some(reply) = msg.reply_to_message() {
                if let Some(photo) = reply.photo() {
                    photo.last().unwrap()
                } else {
                    bot.send_message(msg.chat.id, "No image provided")
                        .reply_to_message_id(msg.id)
                        .await?;
                    return Ok(msg);
                }
            } else {
                bot.send_message(msg.chat.id, "No image provided")
                    .reply_to_message_id(msg.id)
                    .await?;
                return Ok(msg);
            }
        }
    };

    log::info!("Photo: {:?}", photo);

    let file_path = bot.get_file(photo.file.id.clone()).await?.path;
    let mut buf = Vec::new();
    bot.download_file(&file_path, &mut buf).await?;

    let base64_image = BASE64_STANDARD.encode(&buf);

    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    let request_body = &OllamaRequest {
        model: "llava".to_string(),
        prompt,
        stream: false,
        images: Some(vec![base64_image]),
    };

    // Save this request in json to the disk
    // let request_body_json = json!(request_body);
    // let request_body_json = serde_json::to_string_pretty(&request_body_json).unwrap();
    // use std::fs::File;
    // use std::io::Write;
    // let mut file = File::create("request.json").unwrap();
    // file.write_all(request_body_json.as_bytes()).unwrap();

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request_body)
        .send()
        .await;

    match response {
        Ok(response) => {
            let res: Value = response.json().await?;
            // let text = response.text().await?;
            if let Some(response_text) = res["response"].as_str() {
                // log::info!("Response text: {}", response_text);

                bot.send_message(msg.chat.id, response_text)
                    .reply_to_message_id(msg.id)
                    .await
            } else {
                bot.send_message(msg.chat.id, "Error: no response")
                    .reply_to_message_id(msg.id)
                    .await
            }
        }
        Err(e) => {
            log::info!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {}", e))
                .reply_to_message_id(msg.id)
                .await?;

            Err(e.into())
        }
    }
}

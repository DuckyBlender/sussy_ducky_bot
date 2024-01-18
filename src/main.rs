use base64::prelude::*;
use log::{debug, error, info};
use rand::prelude::*;
use reqwest::StatusCode;
use serde_json::Value;
use teloxide::{
    net::Download,
    prelude::*,
    types::{BotCommand, ChatAction, InputFile, True},
    RequestError,
};

mod structs;
use structs::*;

const TTS_VOICES: [&str; 6] = ["alloy", "echo", "fable", "onyx", "nova", "shimmer"];

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    info!("Starting command bot...");

    let bot = Bot::from_env();

    set_commands(&bot).await.unwrap();

    teloxide::repl(bot, handler).await;
}

async fn set_commands(bot: &Bot) -> Result<True, RequestError> {
    let commands = vec![
        BotCommand::new("mistral", "Generate text using Mistral7B"),
        // BotCommand::new("m", "Alias for /mistral"),
        BotCommand::new("llava", "Generate text from image using Llava"),
        // BotCommand::new("l", "Alias for /llava"),
        BotCommand::new("help", "Show available commands"),
        // BotCommand::new("h", "Alias for /help"),
        // BotCommand::new("start", "Start information"),
        BotCommand::new("ping", "Check the bot's latency"),
        BotCommand::new(
            "httpcat",
            "Get an image of a cat for a given HTTP status code",
        ),
        BotCommand::new("tts", "Text to speech using random OpenAI voice"),
        BotCommand::new(
            "caveman",
            "Generate text using Mistral7B in caveman language",
        ),
    ];

    bot.set_my_commands(commands).await
}

fn parse_command(msg: &Message) -> (Option<&str>, Option<&str>) {
    let bot_name = std::env::var("BOT_NAME").unwrap_or("sussy_ducky_bot".to_string());
    let text = msg.text().unwrap_or("");
    let mut iter = text.splitn(2, ' ');
    let command = iter.next();
    let args = iter.next();

    match command {
        Some(command) if command.ends_with(&bot_name) => {
            let command = &command[..command.len() - bot_name.len() - 1]; // -1 to remove @
            (Some(command), args)
        }
        Some(command) if !command.contains("@") => (Some(command), args),
        _ => (None, None),
    }
}

fn parse_command_in_caption(msg: &Message) -> (Option<&str>, Option<&str>) {
    let bot_name = std::env::var("BOT_NAME").unwrap_or("sussy_ducky_bot".to_string());
    let caption = msg.caption().unwrap_or("");
    let mut iter = caption.splitn(2, ' ');
    let command = iter.next();
    let args = iter.next();

    match command {
        Some(command) if command.ends_with(&bot_name) => {
            let command = &command[..command.len() - bot_name.len() - 1]; // -1 to remove @
            (Some(command), args)
        }
        Some(command) if !command.contains("@") => (Some(command), args),
        _ => (None, None),
    }
}

async fn handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    // info!("Received message: {}", msg.text().unwrap_or(""));

    // Check if the message is a message or an image with a caption
    if msg.photo().is_some() && msg.caption().is_some() {
        info!("Message is an image with a caption");
        let (command, args) = parse_command_in_caption(&msg);
        // debug!("Command: {:?}, args: {:?}", command, args);
        match command {
            Some("/llava") | Some("/l") => {
                let prompt = args.unwrap_or("").to_string();
                debug!("Executing llava command with prompt: {}", prompt);
                llava(bot, msg, prompt).await?;
            }
            _ => {}
        }
    } else if msg.text().is_some() {
        // info!("Message is a text message");
        let (command, args) = parse_command(&msg);
        match command {
            Some("/mistral") | Some("/m") => {
                let prompt = args.unwrap_or("").to_string();
                debug!("Executing mistral command with prompt: {}", prompt);
                mistral(bot, msg, prompt, false).await?;
            }
            Some("/caveman") => {
                let prompt = args.unwrap_or("").to_string();
                debug!("Executing caveman command with prompt: {}", prompt);
                mistral(bot, msg, prompt, false).await?;
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
            Some("/ping") => {
                // Ping api.telegram.org and calculate the latency
                let start = std::time::Instant::now();
                let res = reqwest::get("https://api.telegram.org").await;
                let latency = start.elapsed().as_millis();
                match res {
                    Ok(_) => {
                        bot.send_message(msg.chat.id, format!("Pong! Latency: {}ms", latency))
                            .reply_to_message_id(msg.id)
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(msg.chat.id, format!("Error calculating latency: {}", e))
                            .reply_to_message_id(msg.id)
                            .await?;
                    }
                }
            }
            Some("/httpcat") => {
                // Ping http://http.cat/{argument}
                if args.is_none() {
                    bot.send_message(
                        msg.chat.id,
                        "No argument provided: Please provide a status code",
                    )
                    .reply_to_message_id(msg.id)
                    .await?;
                    return Ok(());
                }
                let status_code = args.unwrap();
                let first_argument = status_code.splitn(2, ' ').next().unwrap();
                // Check if it's a 3 digit number
                if first_argument.len() != 3 {
                    bot.send_message(
                        msg.chat.id,
                        "Invalid argument: Please provide a 3 digit status code",
                    )
                    .reply_to_message_id(msg.id)
                    .await?;
                    return Ok(());
                }
                // Download the image
                let res = reqwest::get(format!("https://http.cat/{}", first_argument)).await;
                // Send the image
                match res {
                    Ok(res) => {
                        let body = res.bytes().await?;
                        let buf = body.to_vec();
                        bot.send_photo(msg.chat.id, InputFile::memory(buf))
                            .reply_to_message_id(msg.id)
                            .await?;
                    }
                    Err(e) => {
                        // Check which error it is
                        match e.status() {
                            Some(StatusCode::NOT_FOUND) => {
                                bot.send_message(
                                    msg.chat.id,
                                    format!("Error: {} is not a valid status code", status_code),
                                )
                                .reply_to_message_id(msg.id)
                                .await?;
                            }
                            _ => {
                                bot.send_message(
                                    msg.chat.id,
                                    format!("Error downloading image: {}", e),
                                )
                                .reply_to_message_id(msg.id)
                                .await?;
                            }
                        }
                    }
                }
            }
            Some("/tts") => {
                // Available TTS voices: alloy, echo, fable, onyx, nova, and shimmer
                // Check if there is a prompt after the command
                // If not, check if there is a reply
                // If not, send an error message

                let prompt;
                if args.is_none() {
                    if let Some(reply) = msg.reply_to_message() {
                        if let Some(text) = reply.text() {
                            prompt = text.to_string();
                        } else {
                            bot.send_message(msg.chat.id, "No prompt provided")
                                .reply_to_message_id(msg.id)
                                .await?;
                            return Ok(());
                        }
                    } else {
                        bot.send_message(msg.chat.id, "No prompt provided")
                            .reply_to_message_id(msg.id)
                            .await?;
                        return Ok(());
                    }
                } else {
                    prompt = args.unwrap().to_string();
                }

                // Send typing indicator
                bot.send_chat_action(msg.chat.id, ChatAction::RecordVoice)
                    .await?;

                // Send the request
                let voice = TTS_VOICES.choose(&mut rand::thread_rng()).unwrap();
                let res = reqwest::Client::new()
                    .post("https://api.openai.com/v1/audio/speech")
                    .bearer_auth(std::env::var("OPENAI_KEY").unwrap())
                    .json(&TTSRequest {
                        model: "tts-1".to_string(),
                        input: prompt,
                        voice: voice.to_string(),
                    })
                    .send()
                    .await;

                if res.is_err() {
                    error!("Error sending request: {}", res.as_ref().err().unwrap());
                    bot.send_message(
                        msg.chat.id,
                        format!("Error: {}", res.as_ref().err().unwrap()),
                    )
                    .reply_to_message_id(msg.id)
                    .await?;
                    return Ok(());
                }

                // Check if the request was successful
                if res.as_ref().unwrap().status().is_client_error() {
                    let error = res.unwrap().text().await.unwrap();
                    error!("Error sending request: {}", error);
                    bot.send_message(msg.chat.id, format!("Error: {}", error))
                        .reply_to_message_id(msg.id)
                        .await?;
                    return Ok(());
                }

                bot.send_chat_action(msg.chat.id, ChatAction::RecordVoice)
                    .await?;

                // The response is the audio file in mp3
                // Send the response
                let res = res.unwrap();

                let body = res.bytes().await?;
                let buf = body.to_vec();
                bot.send_voice(msg.chat.id, InputFile::memory(buf))
                    .caption(format!("Voice: {}", voice))
                    .reply_to_message_id(msg.id)
                    .await?;
            }
            _ => {}
        }
    } else {
        // info!("Message is not a text message nor an image with a caption");
    }

    Ok(())
}

async fn mistral(
    bot: Bot,
    msg: Message,
    prompt: String,
    caveman: bool,
) -> Result<Message, RequestError> {
    info!("Starting mistral function");
    // If the prompt is empty, check if there is a reply
    let mut prompt = if prompt.is_empty() {
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

    if caveman {
        prompt = format!("Reply to this message in caveman language. Use all caps. Make many grammatical errors. Message: {}", prompt);
    }

    // Send typing action
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await?;

    // Send the request
    let now = std::time::Instant::now();
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
    let elapsed = now.elapsed().as_secs_f32();

    match res {
        Ok(_) => {
            // info!("Request sent successfully");
        }
        Err(e) => {
            error!("Error sending request: {}", e);
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
            bot.send_message(
                msg.chat.id,
                // round to one decimal place
                format!(
                    "{}\n\nGeneration time: {}s",
                    res.response,
                    (elapsed * 10.0).round() / 10.0
                ),
            )
            .reply_to_message_id(msg.id)
            .await
        }
        Err(e) => {
            error!("Error parsing response: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {}", e))
                .reply_to_message_id(msg.id)
                .await
        }
    }
}

async fn llava(bot: Bot, msg: Message, mut prompt: String) -> Result<Message, RequestError> {
    info!("Starting llava function");

    // info!("Prompt: {}", prompt);

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

    info!("Photo: {:?}", photo);

    let file_path = bot.get_file(photo.file.id.clone()).await?.path;
    let mut buf = Vec::new();
    bot.download_file(&file_path, &mut buf).await?;

    let base64_image = BASE64_STANDARD.encode(&buf);

    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
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
    let now = std::time::Instant::now();
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request_body)
        .send()
        .await;
    let elapsed = now.elapsed().as_secs_f32();

    match response {
        Ok(response) => {
            let res: Value = response.json().await?;
            // let text = response.text().await?;
            if let Some(response_text) = res["response"].as_str() {
                // info!("Response text: {}", response_text);
                let response_text = format!(
                    "{}\n\nGeneration time: {}s",
                    response_text,
                    (elapsed * 10.0).round() / 10.0
                );

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
            info!("Error sending request: {}", e);
            bot.send_message(msg.chat.id, format!("Error: {}", e))
                .reply_to_message_id(msg.id)
                .await?;

            Err(e.into())
        }
    }
}

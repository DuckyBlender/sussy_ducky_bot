use std::fmt;
use teloxide::types::Message;

pub enum ModelType {
    // Ollama (local)
    MistralCaveman,    // caveman-mistral (custom model)
    MistralUncensored, // dolphin-mistral
    Mistral,           // mistral-openorca
    TinyLlama,         // tiny-llama

    // Perplexity (online)
    Mixtral, // mixtral-8x7b-instruct
    Online,  // pplx-7b-online
}

impl fmt::Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ModelType::MistralCaveman => write!(f, "caveman-mistral"),
            ModelType::MistralUncensored => write!(f, "dolphin-mistral"),
            ModelType::Mistral => write!(f, "mistral-openorca"),
            ModelType::TinyLlama => write!(f, "tinyllama"),
            ModelType::Mixtral => write!(f, "mixtral-8x7b-instruct"),
            ModelType::Online => write!(f, "pplx-7b-online"),
        }
    }
}

pub fn parse_command(msg: Message) -> (Option<String>, Option<String>) {
    let bot_name = std::env::var("BOT_NAME").unwrap_or("sussy_ducky_bot".to_string());
    let text = msg.text().unwrap_or("");
    let mut iter = text.splitn(2, ' ');
    let command = iter.next().map(std::string::ToString::to_string);
    let args = iter.next().map(std::string::ToString::to_string);

    match &command {
        Some(command) if command.ends_with(&bot_name) => {
            let command = &command[..command.len() - bot_name.len() - 1]; // -1 to remove @
            (Some(command.to_string()), args)
        }
        Some(command) if !command.contains('@') => (Some(command.to_string()), args),
        _ => (None, None),
    }
}

pub fn parse_command_in_caption(msg: Message) -> (Option<String>, Option<String>) {
    let bot_name = std::env::var("BOT_NAME").unwrap_or("sussy_ducky_bot".to_string());
    let caption = msg.caption().unwrap_or("");
    let mut iter = caption.splitn(2, ' ');
    let command = iter.next().map(std::string::ToString::to_string);
    let args = iter.next().map(std::string::ToString::to_string);

    match &command {
        Some(command) if command.ends_with(&bot_name) => {
            let command = &command[..command.len() - bot_name.len() - 1]; // -1 to remove @
            (Some(command.to_string()), args)
        }
        Some(command) if !command.contains('@') => (Some(command.to_string()), args),
        _ => (None, None),
    }
}

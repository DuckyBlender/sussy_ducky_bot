use teloxide::types::Message;

pub enum ModelType {
    MistralStandard, // mistral
    MistralCaveman,  // still mistral, just with a different prompt
    MistralDolphin,  // dolphin-mistral
    MistralOpenOrca, // mistral-openorca
    TinyLlama,       // tiny-llama
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

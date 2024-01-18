use teloxide::types::Message;

pub fn parse_command(msg: &Message) -> (Option<&str>, Option<&str>) {
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
        Some(command) if !command.contains('@') => (Some(command), args),
        _ => (None, None),
    }
}

pub fn parse_command_in_caption(msg: &Message) -> (Option<&str>, Option<&str>) {
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
        Some(command) if !command.contains('@') => (Some(command), args),
        _ => (None, None),
    }
}

use teloxide::{prelude::*, utils::command::BotCommands};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv::dotenv().ok();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Display this text")]
    Help,
    #[command(description = "Generate using Mistral 7B")]
    Mistral(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    // If the command is in DM, just assume it's a prompt for mistral
    if msg.chat.is_private() {
        bot.send_message(msg.chat.id, "Hello!").await?;
    }

    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Mistral(prompt) => {
            // Make this request with reqwest
            // Example:
            // curl http://localhost:11434/api/generate -d '{
            //   "model": "llama2",
            //   "prompt": "Why is the sky blue?",
            //   "stream": false
            // }'
            bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
                .await?;

            let res = reqwest::Client::new()
                .post("http://localhost:11434/api/generate")
                .json(&serde_json::json!({
                    "model": "mistral",
                    "stream": false, // TODO: edit message every 3 seconds
                    "prompt": prompt
                }))
                .send()
                .await;

            match res {
                Ok(_) => {}
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Error: {}", e))
                        .await?;
                    return Ok(());
                }
            };

            // Parse the response
            // Example:
            // {
            //   "model": "llama2",
            //   "created_at": "2023-08-04T19:22:45.499127Z",
            //   "response": "The sky is blue because it is the color of the sky.",
            //   "done": true,
            //   "context": [1, 2, 3],
            //   "total_duration": 5043500667,
            //   "load_duration": 5025959,
            //   "prompt_eval_count": 26,
            //   "prompt_eval_duration": 325953000,
            //   "eval_count": 290,
            //   "eval_duration": 4709213000
            // }
            let res: serde_json::Value = res.unwrap().json().await?;
            let response = res["response"].as_str().unwrap_or("No response");
            // Send the response
            bot.send_message(msg.chat.id, response)
                .reply_to_message_id(msg.id)
                .await?
        }
    };

    Ok(())
}

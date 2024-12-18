use log::{debug, info};
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use teloxide::{prelude::*, types::{ChatAction, ReplyParameters}, utils::command::BotCommands};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    fern::Dispatch::new()
    // Perform allocation-free log formatting
    .format(|out, message, record| {
        out.finish(format_args!(
            "[{} {} {}] {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.target(),
            message
        ))
    })
    // Add blanket level filter -
    .level(log::LevelFilter::Warn)
    // - and per-module overrides
    .level_for("sussy_ducky_bot", log::LevelFilter::Info)
    // Output to stdout, files, and other Dispatch configurations
    .chain(std::io::stdout())
    .chain(fern::log_file("output.log").unwrap())
    // Apply globally
    .apply()
    .expect("Failed to initialize logging");

    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    // Get bot information  
    let me = bot.get_me().await.expect("Failed to get bot info");  
    info!("Started @{}", me.username());  

    // TODO: Get sqlite connection

    // Get ollama connection
    let ollama = Ollama::default();

    // Start the dispatcher  
    let handler =  
        Update::filter_message()  
            // Handle normal commands  
            .branch(  
                dptree::entry()  
                    .filter_command::<Command>()  
                    .endpoint({
                        let ollama = ollama.clone();
                        move |bot, msg, cmd| {  
                            let ollama = ollama.clone();
                            answer(bot, msg, cmd, ollama)
                        }
                    }),  
            );
            // Handle captions under photos and documents & normal commands for now 
            // .branch(dptree::entry().endpoint(move |bot, msg| {  
            //     let username_translation = translate_telegram_to_quire();  
            //     async move { handle_manual_command(bot, msg, &username_translation, is_testing).await }  
            // }));  

    Dispatcher::builder(bot, handler)  
        .enable_ctrlc_handler()  
        .build()  
        .dispatch()  
        .await;  
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "ask llama 3.2 1b", alias = "l")]
    Llama(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command, ollama: Ollama) -> ResponseResult<()> {
    debug!("Received command: {:?}", cmd);
    match cmd {
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::Llama(prompt) => {
            bot.send_chat_action(msg.chat.id, ChatAction::Typing).await?;
            const MODEL: &str = "llama3.2:1b";
            let res = ollama.generate(GenerationRequest::new(MODEL.to_string(), prompt)).await;
            match res {
                Ok(response) => {
                    bot.send_message(msg.chat.id, response.response).reply_parameters(ReplyParameters::new(msg.id)).await?;
                    return Ok(());
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Error: {}", e)).await?;
                    return Ok(());
                }
            }

        }
    };

    Ok(())
}
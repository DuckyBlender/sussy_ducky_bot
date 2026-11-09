mod history;
mod llm;
mod markdown;
mod prompts;

use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use history::{Entry, History, ImageRef};
use llm::{ChatMessage, Llm};
use prompts::Prompts;
use teloxide::{
    RequestError,
    error_handlers::LoggingErrorHandler,
    net::Download,
    prelude::*,
    types::{ChatAction, Me, MessageId, ParseMode, ReactionType, ReplyParameters, User},
    update_listeners::Polling,
    utils::command::BotCommands,
};
use tokio::sync::Mutex;

/// Reaction used instead of an error message.
const ERROR_EMOJI: &str = "🤯";
/// Reaction for messages there is nothing to answer (e.g. a picture while vision is off).
const NOTHING_EMOJI: &str = "🤷";
/// Telegram allows 4096 characters per message; stay a little below.
const MAX_MESSAGE_CHARS: usize = 4000;
/// Markdown grows when converted to HTML, so it's split into smaller parts.
const MAX_MARKDOWN_CHARS: usize = 3000;
/// Telegram bots can't download files bigger than 20 MB.
const MAX_IMAGE_BYTES: u32 = 20 * 1024 * 1024;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "how to use the bot")]
    Help,
    #[command(hide)]
    Start,
    #[command(description = "ask something (needed in groups): /ask <question>")]
    Ask(String),
    #[command(description = "list system prompts")]
    Prompts,
    #[command(description = "show a prompt: /prompt [name] (yours if no name)")]
    Prompt(String),
    #[command(
        description = "create or edit a prompt: /newprompt <name> <text>, or reply to a message with /newprompt <name>"
    )]
    NewPrompt(String),
    #[command(description = "use a prompt for your messages: /useprompt <name> | default")]
    UsePrompt(String),
    #[command(description = "delete one of your prompts: /delprompt <name>")]
    DelPrompt(String),
}

struct State {
    llm: Llm,
    history: Mutex<History>,
    prompts: Mutex<Prompts>,
    max_chain: usize,
}

#[tokio::main]
async fn main() {
    let data_dir = PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| "data".into()));
    fs::create_dir_all(data_dir.join("logs")).expect("cannot create data directory");
    setup_logging(&data_dir);

    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN is not set");
    let llm_url = env::var("LLM_URL").unwrap_or_else(|_| "http://localhost:8081".into());
    let model = env::var("LLM_MODEL").unwrap_or_else(|_| "bolt".into());
    let prompts = Prompts::load(data_dir.join("prompts.json")).expect("cannot load prompts.json");
    let state = Arc::new(State {
        llm: Llm::new(&llm_url, model),
        history: Mutex::new(History::new(10_000)),
        prompts: Mutex::new(prompts),
        max_chain: env::var("MAX_CHAIN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
    });

    let bot = Bot::new(token);
    let me = bot
        .get_me()
        .await
        .expect("failed to reach Telegram (bad token?)");
    log::info!("Logged in as @{}, using LLM at {llm_url}", me.username());

    match bot.set_my_commands(Command::bot_commands()).await {
        Ok(_) => log::info!("Registered bot commands"),
        Err(e) => log::warn!("Could not register bot commands: {e}"),
    }

    // Skip whatever queued up while the bot was offline instead of answering it all now.
    let listener = Polling::builder(bot.clone())
        .drop_pending_updates()
        .delete_webhook()
        .await
        .build();
    let handler = Update::filter_message().endpoint(handle);
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state, me])
        .default_handler(|_| async {})
        .enable_ctrlc_handler()
        .build()
        .dispatch_with_listener(
            listener,
            LoggingErrorHandler::with_custom_text("update listener error"),
        )
        .await;
}

/// Logs to stdout and to a daily file in `<data_dir>/logs`.
fn setup_logging(data_dir: &Path) {
    let level = env::var("LOG_LEVEL")
        .ok()
        .and_then(|l| l.parse().ok())
        .unwrap_or(log::LevelFilter::Info);
    let log_prefix = data_dir.join("logs").join("sussy_ducky_bot.");
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {:<5} {}: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ));
        })
        .level(log::LevelFilter::Warn)
        .level_for(env!("CARGO_CRATE_NAME"), level)
        .chain(std::io::stdout())
        .chain(fern::DateBased::new(log_prefix, "%Y-%m-%d.log"))
        .apply()
        .expect("failed to set up logging");
}

async fn handle(bot: Bot, msg: Message, me: Me, state: Arc<State>) -> ResponseResult<()> {
    let raw_text = text_of(&msg);
    let command = Command::parse(raw_text, me.username()).ok();
    if !is_addressed(&msg, &me, raw_text, command.is_some()) {
        return Ok(());
    }

    let result = match command {
        None | Some(Command::Ask(_)) => respond(&bot, &msg, &me, &state).await,
        Some(cmd) => match msg.from.as_ref() {
            Some(user) => run_command(&bot, &msg, user, &state, cmd).await,
            None => Ok(()),
        },
    };
    if let Err(e) = result {
        log::error!("chat {} msg {}: {e:#}", msg.chat.id, msg.id);
        react(&bot, &msg, ERROR_EMOJI).await;
    }
    Ok(())
}

/// Private chats: everything. Groups: commands, @mentions and replies to the bot.
fn is_addressed(msg: &Message, me: &Me, text: &str, is_command: bool) -> bool {
    msg.chat.is_private()
        || is_command
        || contains_mention(text, me)
        || msg
            .reply_to_message()
            .and_then(|r| r.from.as_ref())
            .is_some_and(|u| u.id == me.id)
}

/// A message's text, or its caption for media.
fn text_of(msg: &Message) -> &str {
    msg.text().or(msg.caption()).unwrap_or_default()
}

fn contains_mention(text: &str, me: &Me) -> bool {
    let mention = format!("@{}", me.username()).to_lowercase();
    text.to_lowercase().contains(&mention)
}

/// The text with a leading /ask and @mentions of the bot removed.
fn clean_text(text: &str, me: &Me) -> String {
    let text = match Command::parse(text, me.username()) {
        Ok(Command::Ask(question)) => question,
        _ => text.to_owned(),
    };
    let mention = format!("@{}", me.username());
    text.split(' ')
        .filter(|w| !w.eq_ignore_ascii_case(&mention))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

fn display_name(user: &User) -> String {
    user.username
        .as_ref()
        .map_or_else(|| user.first_name.clone(), |u| format!("@{u}"))
}

fn preview(text: &str, max: usize) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() > max {
        format!("{}…", line.chars().take(max).collect::<String>())
    } else {
        line
    }
}

async fn run_command(
    bot: &Bot,
    msg: &Message,
    user: &User,
    state: &State,
    cmd: Command,
) -> Result<()> {
    let uid = user.id.0;
    let mut prompts = state.prompts.lock().await;
    let reply = match cmd {
        Command::Help | Command::Start => help_text(),
        Command::Ask(_) => unreachable!("handled by respond"),
        Command::Prompts => list_prompts(&prompts, uid),
        Command::Prompt(name) => show_prompt(&prompts, uid, name.trim()),
        Command::NewPrompt(args) => new_prompt(&mut prompts, msg, user, args.trim())?,
        Command::UsePrompt(name) => use_prompt(&mut prompts, uid, name.trim())?,
        Command::DelPrompt(name) => del_prompt(&mut prompts, uid, name.trim())?,
    };
    drop(prompts);
    send_text(bot, msg.chat.id, msg.id, &reply, Format::Plain).await?;
    Ok(())
}

fn help_text() -> String {
    format!(
        "Quack! Message me in private, or mention me / reply to me / use /ask in groups. \
         Reply to my messages to continue a conversation.\n\n\
         The default system prompt is \"{}\". Create your own with /newprompt and pick it \
         with /useprompt; your choice applies to all your messages.\n\n{}",
        prompts::DEFAULT_TEXT,
        Command::descriptions()
    )
}

fn list_prompts(prompts: &Prompts, uid: u64) -> String {
    let active = prompts.active(uid).map(|(name, _)| name);
    let mut out = String::new();
    for (name, p) in prompts.list() {
        let mark = if active == Some(name.as_str()) {
            "▶"
        } else {
            "•"
        };
        let _ = writeln!(
            out,
            "{mark} {name} (by {}): {}",
            p.author,
            preview(&p.text, 60)
        );
    }
    if out.is_empty() {
        return "No prompts yet. Create one: /newprompt <name> <text>".into();
    }
    format!(
        "System prompts (▶ = yours):\n{out}\nYou use: {}. Switch with /useprompt <name> | {}",
        active.unwrap_or(prompts::DEFAULT_NAME),
        prompts::DEFAULT_NAME
    )
}

/// Shows the named prompt, or the user's own when no name is given.
fn show_prompt(prompts: &Prompts, uid: u64, name: &str) -> String {
    let found = if name.is_empty() {
        prompts.active(uid)
    } else {
        prompts.get(name)
    };
    match found {
        Some((name, p)) => format!("\"{name}\" (by {}):\n\n{}", p.author, p.text),
        None if name.is_empty() || name.eq_ignore_ascii_case(prompts::DEFAULT_NAME) => {
            format!(
                "\"{}\" (built in):\n\n{}",
                prompts::DEFAULT_NAME,
                prompts::DEFAULT_TEXT
            )
        }
        None => prompts::not_found(name),
    }
}

fn new_prompt(prompts: &mut Prompts, msg: &Message, user: &User, args: &str) -> Result<String> {
    let (name, text) = args
        .split_once(char::is_whitespace)
        .map_or((args, ""), |(name, text)| (name, text.trim()));
    // "/newprompt <name>" sent as a reply takes the replied-to message as the text.
    let text = if text.is_empty() {
        msg.reply_to_message().map(text_of).unwrap_or_default()
    } else {
        text
    };
    if name.is_empty() || text.is_empty() {
        return Ok(
            "Usage: /newprompt <name> <text>, or reply to a message with /newprompt <name>".into(),
        );
    }
    Ok(
        match prompts.upsert(name, user.id.0, display_name(user), text) {
            Ok((name, updated)) => {
                prompts.save()?;
                log::info!("user {} saved prompt {name}", user.id);
                let verb = if updated { "Updated" } else { "Created" };
                format!("{verb} \"{name}\". Use it with /useprompt {name}")
            }
            Err(refusal) => refusal,
        },
    )
}

fn use_prompt(prompts: &mut Prompts, uid: u64, name: &str) -> Result<String> {
    if name.is_empty() {
        return Ok(format!(
            "Usage: /useprompt <name>, or /useprompt {}. See /prompts.",
            prompts::DEFAULT_NAME
        ));
    }
    let reply = if name.eq_ignore_ascii_case(prompts::DEFAULT_NAME) {
        prompts.clear(uid);
        "You now use the default prompt.".to_owned()
    } else {
        match prompts.select(uid, name) {
            Ok(name) => format!("You now use \"{name}\"."),
            Err(refusal) => return Ok(refusal),
        }
    };
    prompts.save()?;
    Ok(reply)
}

fn del_prompt(prompts: &mut Prompts, uid: u64, name: &str) -> Result<String> {
    if name.is_empty() {
        return Ok("Usage: /delprompt <name>".into());
    }
    Ok(match prompts.delete(name, uid) {
        Ok(name) => {
            prompts.save()?;
            log::info!("user {uid} deleted prompt {name}");
            format!("Deleted \"{name}\".")
        }
        Err(refusal) => refusal,
    })
}

/// The highest quality image attached to the message, if any.
fn best_image(msg: &Message) -> Option<ImageRef> {
    if let Some(photo) = msg.photo().and_then(|sizes| {
        sizes
            .iter()
            .filter(|p| p.file.size <= MAX_IMAGE_BYTES)
            .max_by_key(|p| u64::from(p.width) * u64::from(p.height))
    }) {
        return Some(ImageRef {
            file_id: photo.file.id.clone(),
            mime: "image/jpeg".into(),
        });
    }
    // Images sent "as file" are uncompressed originals.
    let doc = msg.document()?;
    let mime = doc.mime_type.as_ref()?.essence_str().to_owned();
    (mime.starts_with("image/") && doc.file.size <= MAX_IMAGE_BYTES).then(|| ImageRef {
        file_id: doc.file.id.clone(),
        mime,
    })
}

fn entry_from(msg: &Message, me: &Me) -> Entry {
    let from_bot = msg.from.as_ref().is_some_and(|u| u.id == me.id);
    let text = text_of(msg);
    Entry {
        from_bot,
        text: if from_bot {
            text.to_owned()
        } else {
            clean_text(text, me)
        },
        image: best_image(msg),
        parent: msg.reply_to_message().map(|r| r.id),
    }
}

async fn respond(bot: &Bot, msg: &Message, me: &Me, state: &State) -> Result<()> {
    let chat = msg.chat.id;

    // Remember this message (and the one it replies to, in case we never saw it)
    // and walk the reply chain back to its start.
    let chain = {
        let mut history = state.history.lock().await;
        if let Some(parent) = msg.reply_to_message()
            && !history.contains(chat, parent.id)
        {
            let mut e = entry_from(parent, me);
            e.parent = None; // Telegram doesn't nest replies, so the chain ends here.
            history.insert(chat, parent.id, e);
        }
        history.insert(chat, msg.id, entry_from(msg, me));
        history.chain(chat, msg.id, state.max_chain)
    };

    // The sender's chosen system prompt, or the default.
    let system = match &msg.from {
        Some(u) => state
            .prompts
            .lock()
            .await
            .active(u.id.0)
            .map(|(_, p)| p.text.clone()),
        None => None,
    }
    .unwrap_or_else(|| prompts::DEFAULT_TEXT.to_owned());

    let typing = tokio::spawn({
        let bot = bot.clone();
        async move {
            loop {
                let _ = bot.send_chat_action(chat, ChatAction::Typing).await;
                tokio::time::sleep(Duration::from_secs(4)).await;
            }
        }
    });
    let result = generate(bot, state, &system, chain).await;
    typing.abort();

    let Some(reply) = result? else {
        react(bot, msg, NOTHING_EMOJI).await;
        return Ok(());
    };

    let sent = send_text(bot, chat, msg.id, &reply, Format::Markdown).await?;
    let mut history = state.history.lock().await;
    let mut parent = msg.id;
    for (id, chunk) in sent {
        history.insert(
            chat,
            id,
            Entry {
                from_bot: true,
                text: chunk,
                image: None,
                parent: Some(parent),
            },
        );
        parent = id;
    }
    log::info!(
        "chat {chat}: answered msg {} ({} chars)",
        msg.id,
        reply.len()
    );
    Ok(())
}

/// Builds the conversation and asks the model. `None` when there's nothing to answer.
async fn generate(
    bot: &Bot,
    state: &State,
    system: &str,
    chain: Vec<Entry>,
) -> Result<Option<String>> {
    let vision = chain.iter().any(|e| e.image.is_some()) && state.llm.supports_vision().await;

    let mut messages = Vec::with_capacity(chain.len());
    for entry in chain {
        let mut images = Vec::new();
        if let (true, Some(img)) = (vision, &entry.image) {
            images.push(download_image(bot, img).await?);
        }
        if entry.text.is_empty() && images.is_empty() {
            continue;
        }
        messages.push(ChatMessage {
            role: if entry.from_bot { "assistant" } else { "user" },
            text: entry.text,
            images,
        });
    }

    // The model must get the last word from the user.
    if messages.last().is_none_or(|m| m.role != "user") {
        return Ok(None);
    }
    state.llm.chat(system, &messages).await.map(Some)
}

async fn download_image(bot: &Bot, img: &ImageRef) -> Result<String> {
    let file = bot
        .get_file(img.file_id.clone())
        .await
        .context("get_file")?;
    let mut buf = Vec::with_capacity(file.size as usize);
    bot.download_file(&file.path, &mut buf)
        .await
        .context("downloading image")?;
    Ok(format!("data:{};base64,{}", img.mime, BASE64.encode(buf)))
}

#[derive(Clone, Copy)]
enum Format {
    Plain,
    /// Rendered as Telegram HTML, falling back to plain text if Telegram rejects it.
    Markdown,
}

/// Sends `text` split into Telegram-sized parts, each replying to the previous one.
/// Returns the sent message ids with their (unrendered) text.
async fn send_text(
    bot: &Bot,
    chat: ChatId,
    mut reply_to: MessageId,
    text: &str,
    format: Format,
) -> Result<Vec<(MessageId, String)>> {
    let max_chars = match format {
        Format::Plain => MAX_MESSAGE_CHARS,
        Format::Markdown => MAX_MARKDOWN_CHARS,
    };
    let mut sent = Vec::new();
    for chunk in split_message(text, max_chars) {
        let html = match format {
            Format::Plain => None,
            Format::Markdown => Some(markdown::to_telegram_html(&chunk)),
        };
        let result = match html {
            Some(html) => match send_part(bot, chat, reply_to, html, Some(ParseMode::Html)).await {
                Err(e) => {
                    log::warn!("Telegram rejected HTML, sending plain text: {e}");
                    send_part(bot, chat, reply_to, chunk.clone(), None).await
                }
                ok => ok,
            },
            None => send_part(bot, chat, reply_to, chunk.clone(), None).await,
        };
        let m = result.context("sending message")?;
        reply_to = m.id;
        sent.push((m.id, chunk));
    }
    Ok(sent)
}

async fn send_part(
    bot: &Bot,
    chat: ChatId,
    reply_to: MessageId,
    text: String,
    parse_mode: Option<ParseMode>,
) -> Result<Message, RequestError> {
    let request = bot
        .send_message(chat, text)
        .reply_parameters(ReplyParameters::new(reply_to).allow_sending_without_reply());
    match parse_mode {
        Some(mode) => request.parse_mode(mode).await,
        None => request.await,
    }
}

async fn react(bot: &Bot, msg: &Message, emoji: &str) {
    let reaction = ReactionType::Emoji {
        emoji: emoji.to_owned(),
    };
    if let Err(e) = bot
        .set_message_reaction(msg.chat.id, msg.id)
        .reaction(vec![reaction])
        .await
    {
        log::warn!("could not react to msg {}: {e}", msg.id);
    }
}

/// Splits long text into chunks of at most `max_chars`, preferring line breaks.
fn split_message(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut rest = text;
    while rest.chars().count() > max_chars {
        let limit = rest
            .char_indices()
            .nth(max_chars)
            .map_or(rest.len(), |(i, _)| i);
        let cut = rest[..limit]
            .rfind('\n')
            .or_else(|| rest[..limit].rfind(' '))
            .filter(|&i| i > 0)
            .unwrap_or(limit);
        chunks.push(rest[..cut].trim_end().to_owned());
        rest = rest[cut..].trim_start();
    }
    if !rest.is_empty() {
        chunks.push(rest.to_owned());
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_long_text() {
        let text = "word ".repeat(2000);
        let chunks = split_message(&text, MAX_MESSAGE_CHARS);
        assert_eq!(chunks.len(), 3);
        assert!(
            chunks
                .iter()
                .all(|c| c.chars().count() <= MAX_MESSAGE_CHARS)
        );
        assert_eq!(chunks.concat().replace(' ', ""), text.replace(' ', ""));
    }

    #[test]
    fn short_text_is_one_chunk() {
        assert_eq!(split_message("hi", MAX_MESSAGE_CHARS), vec!["hi"]);
    }

    #[test]
    fn parses_commands() {
        let p = |t| Command::parse(t, "DuckyBot");
        assert!(matches!(p("/ask"), Ok(Command::Ask(a)) if a.is_empty()));
        assert!(matches!(p("/ask@DuckyBot hi there"), Ok(Command::Ask(a)) if a == "hi there"));
        assert!(p("/ask@OtherBot hi").is_err());
        assert!(
            matches!(p("/newprompt pirate Talk\nlike a pirate"), Ok(Command::NewPrompt(a)) if a == "pirate Talk\nlike a pirate")
        );
        assert!(matches!(p("/useprompt default"), Ok(Command::UsePrompt(a)) if a == "default"));
        assert!(p("hello").is_err());
    }
}

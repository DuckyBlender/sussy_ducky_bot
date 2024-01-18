use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use teloxide::requests::ResponseResult;

use teloxide::{types::Message, Bot};

const HELP_TEXT: &str = r#"
⚠️ THIS BOT CAN BE QUITE SLOW, PLEASE BE PATIENT ⚠️
Btw, this bot is open source! Check it out at https://github.com/DuckyBlender/sussy_ducky_bot
Oh and the bot works with replies too! (for example you can reply to a photo with /llava)

Available commands:
=-= TEXT =-=
/mistral or /m: generate text using mistral LLM
/caveman: generate text using mistral LLM in caveman language
/dolphin: generate text using dolphin-mistral. This should be a more uncensored version of mistral
/orca: generate text using mistral-openorca. This should be a more smart version of mistral (experimental).

=-= IMAGE RECOGNITION =-=
/llava or /l: generate text from image using llava LLM

=-= IMAGE GENERATION =-=
soon :P

=-= AUDIO =-=
/tts: generate audio from text using OpenAI. Random voice is used.

=-= OTHER =-=
/ping: check if the bot is alive
/help: show this message
/start: show a welcome message
/httpcat: get a http cat image

"#;

pub async fn help(bot: Bot, msg: Message) -> ResponseResult<Message> {
    bot.send_message(msg.chat.id, HELP_TEXT)
        .reply_to_message_id(msg.id)
        .await
}

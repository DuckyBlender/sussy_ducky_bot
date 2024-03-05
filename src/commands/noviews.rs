use scraper::{Html, Selector};
use teloxide::{prelude::*, RequestError};

use teloxide::{types::Message, Bot};

pub async fn noviews(bot: Bot, msg: Message) -> Result<Message, RequestError> {
    // Fetch the HTML content
    let url = "https://petittube.com/";
    let body = reqwest::get(url).await.unwrap().text().await.unwrap();

    // Parse HTML with scraper
    let document = Html::parse_document(&body);

    // Define a CSS selector for the iframe
    let selector = Selector::parse("iframe").unwrap();

    if document.select(&selector).next().is_none() {
        return bot.send_message(
            msg.chat.id,
            "There was a problem fetching a video. Please try later.",
        )
        .reply_to_message_id(msg.id)
        .await;
    }

    // Extract iframe attributes
    for iframe in document.select(&selector) {
        // Get the iframe source attribute
        let src_attribute = iframe.value().attr("src");

        if src_attribute.is_none() {
            return bot
                .send_message(
                    msg.chat.id,
                    "There was a problem fetching a video. Please try later.",
                )
                .reply_to_message_id(msg.id)
                .await;
        }

        let src = src_attribute.unwrap();
        return bot
            .send_message(msg.chat.id, src)
            .reply_to_message_id(msg.id)
            .await;
    }

    return bot
        .send_message(
            msg.chat.id,
            "There was a problem fetching a video. Please try later.",
        )
        .reply_to_message_id(msg.id)
        .await;
}
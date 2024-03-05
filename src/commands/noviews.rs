use scraper::{Html, Selector};
use teloxide::payloads::SendMessageSetters;

use teloxide::{requests::Requester, types::Message, Bot, RequestError};

pub async fn noviews(bot: Bot, msg: Message) -> Result<(), RequestError> {
    // Fetch the HTML content
    let url = "https://petittube.com/";
    let body = reqwest::get(url).await.unwrap().text().await.unwrap();

    // Parse HTML with scraper
    let document = Html::parse_document(&body);

    // Define a CSS selector for the iframe
    let selector = Selector::parse("iframe").unwrap();

    if document.select(&selector).next().is_none() {
        bot.send_message(
            msg.chat.id,
            "There was a problem fetching a video. Please try later.",
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    // Extract iframe attributes
    if let Some(iframe) = document.select(&selector).next() {
        // Get the iframe source attribute
        let src_attribute = iframe.value().attr("src");

        if src_attribute.is_none() {
            bot.send_message(
                msg.chat.id,
                "There was a problem fetching a video. Please try later.",
            )
            .reply_to_message_id(msg.id)
            .await?;
            return Ok(());
        }

        let src = src_attribute.unwrap();
        bot.send_message(msg.chat.id, src)
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }

    bot.send_message(
        msg.chat.id,
        "There was a problem fetching a video. Please try later.",
    )
    .reply_to_message_id(msg.id)
    .await?;
    Ok(())
}

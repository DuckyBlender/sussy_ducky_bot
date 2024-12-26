use log::{error, info};
use teloxide::prelude::Requester;
use std::error::Error;
use std::path::PathBuf;
use teloxide::{net::Download, types::File as TelegramFile};
use teloxide::Bot;
use tokio::fs;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub async fn download_telegram_image(
    bot: &Bot,
    file: &TelegramFile,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let mut buf = Vec::new();
    // Get file path
    let path = bot.get_file(file.id.clone()).await?.path;
    bot.download_file(&path, &mut buf).await?;
    Ok(buf)
}

pub fn encode_image_base64(image_data: &[u8]) -> String {
    BASE64.encode(image_data)
}

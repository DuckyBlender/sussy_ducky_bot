use teloxide::payloads::SendMessageSetters;

use teloxide::{requests::Requester, types::Message, Bot, RequestError};

pub async fn noviews(bot: Bot, msg: Message) -> Result<(), RequestError> {
    // Fetch the HTML content
    let url = "https://petittube.com/";
    let body = reqwest::get(url).await.unwrap().text().await.unwrap();

    // Search for the video
    // <body bgcolor=000000><div align=center><br/><a href="https://petittube.com"><img src="title.png"/></a><br/><iframe width="630" height="473" src="https://www.youtube.com/embed/r8cvg4Cby68?version=3&f=videos&app=youtube_gdata&autoplay=1" frameborder="0" allowfullscreen></iframe><br/><br/></div></body>
    
    // Split by "<iframe width="630" height="473" src=" to get the video URL
    let video_split: Vec<&str> = body.split("<iframe width=\"630\" height=\"473\" src=\"").collect();
    let video = video_split[1].split("\"").collect::<Vec<&str>>()[0];
    // Extract this r8cvg4Cby68
    let video = video.split("/").collect::<Vec<&str>>()[4];
    let video = video.split("?").collect::<Vec<&str>>()[0];

    let video = "https://www.youtube.com/watch?v=".to_string() + video; 

    // Send the video
    bot.send_message(msg.chat.id, video)
        .reply_to_message_id(msg.id)
        .await?;
    Ok(())
}

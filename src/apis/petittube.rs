pub async fn get_noviews() -> Result<String, reqwest::Error> {
    // Fetch the HTML content from the specified URL
    let url = "https://petittube.com/";
    let body = reqwest::get(url).await.unwrap().text().await.unwrap();

    // Search for the video URL in the HTML content
    // The video URL is located between "<iframe width="630" height="473" src=" and the next double quote
    let video_split: Vec<&str> = body
        .split("<iframe width=\"630\" height=\"473\" src=\"")
        .collect();
    let video = video_split[1].split('"').collect::<Vec<&str>>()[0];

    // Extract the video ID from the video URL
    // The video ID is located between the fourth and fifth slash ("/") in the URL
    let video = video.split('/').collect::<Vec<&str>>()[4];
    let video = video.split('?').collect::<Vec<&str>>()[0];

    // Construct the complete YouTube video URL
    let video = "https://www.youtube.com/watch?v=".to_string() + video;

    // Send the video URL as a message
    Ok(video)
}

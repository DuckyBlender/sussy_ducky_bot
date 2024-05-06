use reqwest::Client;

async fn get_httpcat(cat: u16) -> Result<Vec<u8>, reqwest::Error> {
    let url = format!("https://http.cat/{}", cat);
    let client = Client::new();
    let body = client.get(url).send().await?.bytes().await?;
    Ok(body.to_vec())
}
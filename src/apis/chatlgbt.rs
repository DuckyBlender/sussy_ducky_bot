use reqwest::Client;

pub async fn chatlgbt_query(prompt: Option<String>) -> Result<String, reqwest::Error> {
    let url = "https://chatlgbtapi.bemani.radom.pl/";
    let client = Client::new();
    let body = client
        .post(url)
        .body(format!("input={}", prompt.unwrap_or_default()))
        .send()
        .await?
        .text()
        .await?;
    Ok(body)
}
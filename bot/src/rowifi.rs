use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize)]
struct RowifiResponse {
    discord_id: String,
    guild_id: String,
    reverse_search_consent: bool,
    roblox_id: u64,
}

const ROWIFI_BASE_URL: &str = "https://api.rowifi.xyz/v2";

pub async fn get_user(discord_id: u64, guild_id: u64, rowifi_token: &str) -> Result<u64> {
    let client = Client::new();
    let get_user_url = format!("{ROWIFI_BASE_URL}/guilds/{guild_id}/members/{discord_id}");
    let token_format = format!("Bot {rowifi_token}");
    let response = client
        .get(get_user_url)
        .header("Authorization", token_format)
        .send()
        .await?;

    let wifi_response = response.json::<RowifiResponse>().await?;

    Ok(wifi_response.roblox_id)
}

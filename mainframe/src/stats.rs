// handles all stat collecting, such as activity lb
//
use crate::{database, discord};
use anyhow::Result;
use chrono::Utc;

pub async fn weekly_activity_lb(webhook: String, url: String, token: String) -> Result<()> {
    let top_10_names = database::get_top(url, token, 10).await?;
    println!("got top {top_10_names:?}");

    let description = top_10_names
        .iter()
        .map(|(name, events)| format!("[**{}**] - {}", events, name))
        .collect::<Vec<String>>()
        .join("\n");

    let embed = discord::Embed {
        title: "This weeks top 10 events attended".to_string(),
        description,
        color: 0xaa0000,
        timestamp: Utc::now().to_rfc3339(),
    };

    discord::activity_lb(webhook, embed).await
}

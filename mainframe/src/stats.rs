// handles all stat collecting, such as activity lb
//
use crate::{database, discord};
use anyhow::Result;
use chrono::Utc;

pub async fn weekly_activity_lb(
    webhook: String,
    url: String,
    token: String,
    cached: &Vec<(String, i32)>,
) -> Result<Vec<(String, i32)>> {
    let top_10_names = database::get_top(url, token, 10).await?;

    if top_10_names == *cached {
        return Ok(top_10_names);
    }
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

    let _ = discord::activity_lb(webhook, embed).await;

    Ok(top_10_names)
}

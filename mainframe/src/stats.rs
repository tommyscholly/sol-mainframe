// handles all stat collecting, such as activity lb
//
use crate::{database, discord};
use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use serde_json::to_string;

#[derive(Serialize)]
struct Embed {
    description: String,
    color: String,
    footer: String,
}

pub async fn weekly_activity_lb(url: String, token: String, webhook: String) -> Result<()> {
    let top_10_names = database::get_top(url, token, 10).await?;

    let description = top_10_names
        .iter()
        .map(|(name, events)| format!("[{}] - {}", events, name))
        .collect::<Vec<String>>()
        .join("\n");

    let embed = Embed {
        description,
        color: "2247400".to_string(),
        footer: Utc::now().to_rfc3339(),
    };

    discord::activity_lb(webhook, to_string(&embed)?).await
}

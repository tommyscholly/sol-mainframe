use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sol_util::mainframe::EventJsonBody;

#[derive(Deserialize, Serialize)]
struct WebhookBody {
    content: String,
    embeds: Vec<String>,
}

pub async fn log_event(event: EventJsonBody, webhook: String, names: &[String]) -> Result<()> {
    let client = Client::new();
    let attendance_str = names.join(", ");
    let now = Utc::now();
    let content = format!(
        "New Event Logged:\n\tHost: {}\n\tAttendance: [{}]\n\tDate: {}\n\tLocation: {}, Kind: {}",
        event.host,
        attendance_str,
        now.to_rfc2822(),
        event.location,
        event.kind
    );
    let body = WebhookBody {
        content,
        embeds: vec![],
    };
    let _ = client.post(webhook).json(&body).send().await?;
    Ok(())
}

pub async fn activity_lb(webhook: String, embed: String) -> Result<()> {
    let client = Client::new();

    let body = WebhookBody {
        content: "Events Top 10".to_string(),
        embeds: vec![embed],
    };

    let _ = client.post(webhook).json(&body).send().await?;
    Ok(())
}

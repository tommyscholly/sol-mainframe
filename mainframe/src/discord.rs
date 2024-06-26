use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde::Serialize;
use sol_util::mainframe::EventJsonBody;

#[derive(Serialize)]
pub struct Embed {
    pub title: String,
    pub description: String,
    pub color: u64,
    pub timestamp: String,
}

#[derive(Serialize)]
struct WebhookBody {
    content: String,
    embeds: Vec<Embed>,
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
        embeds: Vec::new(),
    };
    let _ = client.post(webhook).json(&body).send().await?;
    Ok(())
}

pub async fn activity_lb(webhook: String, embed: Embed) -> Result<()> {
    let client = Client::new();

    let body = WebhookBody {
        content: "".to_string(),
        embeds: vec![embed],
    };

    let res = client.post(webhook).json(&body).send().await?;
    if !res.status().is_success() {
        println!("{} : {}", res.status(), res.text().await.unwrap());
    }
    Ok(())
}

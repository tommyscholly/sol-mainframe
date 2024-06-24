use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sol_util::mainframe::Event;

#[derive(Deserialize, Serialize)]
struct WebhookBody {
    content: String,
}

pub async fn log_event(event: Event, webhook: String) -> Result<()> {
    let client = Client::new();
    let attendance_str = event
        .attendance
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let content = format!(
        "New Event Logged:\n\tHost: {}\n\tAttendance: [{}]\n\tDate: {}\n\tLocation: {}, Kind: {}",
        event.host,
        attendance_str,
        event.event_date.to_rfc2822(),
        event.location,
        event.kind
    );
    let body = WebhookBody { content };
    let _ = client.post(webhook).json(&body).send().await?;
    Ok(())
}

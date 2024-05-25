use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};

use libsql::Builder;
use toml::Table;

use std::fs::read_to_string;

mod database;
mod event;
mod profile;
mod rank;
mod roblox;
mod util;

#[derive(Clone)]
struct AppState {
    token: String,
    url: String,
}

async fn get_profile(State(_state): State<AppState>, Path(_id): Path<u64>) {
    todo!()
}

async fn put_event(
    State(state): State<AppState>,
    Json(body): Json<event::EventJsonBody>,
) -> StatusCode {
    let db = Builder::new_remote(state.url, state.token)
        .build()
        .await
        .unwrap();
    let conn = db.connect().unwrap();

    let event = event::Event::from_json_body(body);

    let attendance_string = serde_json::to_string(&event.attendance).unwrap();
    conn.execute("INSERT INTO events (host, attendance, event_date, kind, location) VALUES (?1, ?2, ?3, ?4, ?5)", (
        event.host,
        attendance_string,
        event.event_date.to_rfc3339(),
        event.kind.as_str(),
        event.location.as_str(),
    )).await.unwrap();

    event.log_attendance(conn).await;

    StatusCode::OK
}

// gets the hosted events from the specified userid
async fn get_hosted(
    State(state): State<AppState>,
    Path(host_id): Path<u64>,
) -> Json<Vec<event::Event>> {
    let db = Builder::new_remote(state.url, state.token)
        .build()
        .await
        .unwrap();
    let conn = db.connect().unwrap();

    let mut rows = conn
        .query("SELECT * FROM events WHERE host = ?1", [host_id])
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Ok(Some(r)) = rows.next().await {
        events.push(event::Event::from_row(&r))
    }

    Json(events)
}

#[tokio::main]
async fn main() {
    let secrets = read_to_string("Secrets.toml").expect("Secrets.toml does not exist");
    let secrets_table = secrets.parse::<Table>().unwrap();

    let db_token_string = secrets_table.get("DB_TOKEN").unwrap().to_string();
    let db_url_string = secrets_table.get("DB_URL").unwrap().to_string();

    let db_token = util::strip_token(db_token_string);
    let db_url = util::strip_token(db_url_string);

    let state = AppState {
        token: db_token,
        url: db_url,
    };

    let app = Router::new()
        .route("/profiles/:id", get(get_profile))
        .route("/events/:id", get(get_hosted))
        .route("/events", put(put_event))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

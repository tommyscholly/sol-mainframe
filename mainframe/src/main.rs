use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};

use libsql::{Builder, Connection};
use sol_util::{
    mainframe::{CreateProfileBody, Event, EventJsonBody, Profile},
    roblox,
};
use tokio::sync::Mutex;
use toml::Table;

use std::{fs, future::IntoFuture, sync::Arc};

mod database;
mod discord;
mod event;
mod event_queue;
mod stats;
mod util;

#[derive(Clone)]
struct AppState {
    token: String,
    url: String,
    webhook: String,      // for admin server
    main_webhook: String, // for main group
    event_queue: Arc<Mutex<event_queue::EventQueue>>,
}

pub async fn get_db_conn(url: String, token: String) -> anyhow::Result<Connection> {
    let db = Builder::new_remote(url, token).build().await?;
    Ok(db.connect()?)
}

async fn get_profile(
    State(state): State<AppState>,
    Path(user_id): Path<u64>,
) -> Json<Option<Profile>> {
    println!("Retrieving profile for {user_id}");
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let sol_rank_id = match roblox::get_rank_in_group(roblox::SOL_GROUP_ID, user_id).await {
        Ok(None) => {
            println!("Profile {user_id} retrieval failed, not in SOL");
            return Json(None);
        }
        Ok(Some((id, _))) => id,
        // this error is probably a timeout, we can normally ignore it
        Err(_e) => 999,
    };

    let (mut profile, in_db) = database::get_profile(user_id, sol_rank_id, &conn).await;
    if in_db {
        println!("Retrieved {profile:?}");
        let mut update = false;
        if sol_rank_id != 999 && profile.try_update_rank(sol_rank_id) {
            update = true;
        }
        if profile.try_reset_events() {
            update = true;
        }
        if update {
            let _ = database::update_profile(profile.clone(), in_db, conn.into()).await;
        }
        Json(Some(profile))
    } else {
        println!("No profile found, creating for {user_id}");
        // ignoring error
        let _ = database::update_profile(profile.clone(), in_db, conn.into()).await;
        Json(Some(profile))
    }
}

async fn create_profile(
    State(state): State<AppState>,
    Json(body): Json<CreateProfileBody>,
) -> StatusCode {
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let mut new_profile = Profile::new(body.user_id, Some(body.username), body.rank_id);
    new_profile.events_attended_this_week = body.events;
    new_profile.total_marks = body.marks;
    new_profile.marks_at_current_rank = body.marks;

    let _ = database::update_profile(new_profile, false, conn.into()).await;

    println!("Created profile for {}", body.user_id);
    StatusCode::OK
}

async fn update_profiles(State(state): State<AppState>) -> StatusCode {
    tokio::spawn(async move {
        match database::update_all(state.url, state.token).await {
            Ok(_) => println!("Updated profiles successfully"),
            Err(e) => eprintln!("Failed to update profiles with {e}"),
        }
    });

    StatusCode::OK
}

async fn increment_events(
    State(state): State<AppState>,
    Path((user_id, increment)): Path<(u64, i32)>,
) -> StatusCode {
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let sol_rank_id = match roblox::get_rank_in_group(roblox::SOL_GROUP_ID, user_id).await {
        Ok(None) => {
            println!("Profile {user_id} retrieval failed, not in SOL");
            return StatusCode::NOT_FOUND;
        }
        Ok(Some((id, _))) => id,
        Err(_e) => 999,
    };
    let (mut profile, in_db) = database::get_profile(user_id, sol_rank_id, &conn).await;

    profile.try_reset_events();
    if sol_rank_id != 999 {
        profile.try_update_rank(sol_rank_id);
    }

    profile.events_attended_this_week += increment;

    profile.try_award_mark();

    if let Err(e) = database::update_profile(profile, in_db, conn.into()).await {
        eprintln!("Failed to update profile {}, with error {}", user_id, e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

async fn get_attended(State(state): State<AppState>, Path(user_id): Path<u64>) -> Json<u64> {
    println!("Counting events attended for {user_id}");
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let count = database::get_attended(user_id, conn).await;
    println!("{user_id} has attended {count} events");
    Json(count)
}

async fn get_events_attended(
    State(state): State<AppState>,
    Path(user_id): Path<u64>,
) -> Json<Vec<u64>> {
    println!("Retrieving event ids for user {user_id}");
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let events = database::get_events_attended(user_id, conn).await;
    println!("{user_id} has attended {events:?}");
    Json(events)
}

async fn get_event_info_by_info(
    State(state): State<AppState>,
    Path(event_id): Path<i32>,
) -> Json<Option<Event>> {
    println!("Getting event {event_id}");
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let event = database::get_event(event_id, conn).await.unwrap_or(None);
    println!("Got event {event:?}");
    Json(event)
}

async fn get_promotable(State(state): State<AppState>) -> Json<Vec<u64>> {
    println!("Getting promotable users");
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let users = database::get_promotable(conn).await.unwrap_or(Vec::new());
    println!("Promotable users {users:?}");
    Json(users)
}

async fn put_event(State(state): State<AppState>, Json(body): Json<EventJsonBody>) -> StatusCode {
    println!(
        "Processing event hosted by {} at {}",
        body.host, body.location
    );
    let mut event_queue = state.event_queue.lock().await;
    let _ = tokio::join!(
        discord::log_event(body.clone(), state.webhook, &body.names),
        discord::log_event(body.clone(), state.main_webhook, &body.names),
    );

    println!("Placed event in queue {body:?}");
    event_queue.push(body);

    StatusCode::OK
}

// gets the hosted events from the specified userid
async fn get_hosted(State(state): State<AppState>, Path(host_id): Path<u64>) -> Json<Vec<Event>> {
    println!("Retrieving events hosted by {host_id}");
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let mut rows = conn
        .query("SELECT * FROM events WHERE host = ?1", [host_id])
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Ok(Some(r)) = rows.next().await {
        events.push(Event::from_row(&r))
    }

    println!("Successfully retrieved events for {host_id}");
    Json(events)
}

#[tokio::main]
async fn main() {
    let secrets = fs::read_to_string("Secrets.toml").expect("Secrets.toml does not exist");
    let secrets_table = secrets.parse::<Table>().unwrap();

    let db_token_string = secrets_table.get("DB_TOKEN").unwrap().to_string();
    let db_url_string = secrets_table.get("DB_URL").unwrap().to_string();
    let webhook_string = secrets_table.get("EVENT_WEBHOOK").unwrap().to_string();
    let main_webhook_string = secrets_table.get("MAIN_EVENT_WEBHOOK").unwrap().to_string();

    let db_token = util::strip_token(db_token_string);
    let db_url = util::strip_token(db_url_string);
    let event_webhook = util::strip_token(webhook_string);
    let main_event_webhook = util::strip_token(main_webhook_string);

    let event_queue = Arc::new(Mutex::new(event_queue::EventQueue::new()));
    let state = AppState {
        token: db_token.clone(),
        url: db_url.clone(),
        webhook: event_webhook,
        main_webhook: main_event_webhook,
        event_queue: event_queue.clone(),
    };

    let app = Router::new()
        .route("/profiles/:id", get(get_profile))
        .route("/profiles/promotable", get(get_promotable))
        .route("/profiles/create", post(create_profile))
        .route("/profiles/update", post(update_profiles))
        .route("/profiles/increment/:id/:inc", post(increment_events))
        .route("/events/:id", get(get_hosted))
        .route("/events", put(put_event))
        .route("/events/attended/:id", get(get_events_attended))
        .route("/events/num-attended/:id", get(get_attended))
        .route("/events/info/:id", get(get_event_info_by_info))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let event_loop = event_queue::queue_loop(event_queue, db_url, db_token);
    println!("SOL Mainframe Listening on 0.0.0.0:3000");
    let _ = tokio::join!(
        axum::serve(listener, app).into_future(),
        event_loop.into_future()
    );
}

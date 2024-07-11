use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{from_fn, Next},
    response::Response,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};

use cosmetics::CosmeticUserInfo;
use libsql::{Builder, Connection};
use sol_util::{
    mainframe::{
        CreateProfileBody, Event, EventJsonBody, EventKind, IncEventBody, Pathway, Profile,
        Progress,
    },
    roblox,
};
use tokio::sync::Mutex;
use toml::Table;

use std::{fs, future::IntoFuture, sync::Arc};

mod cosmetics;
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
    cosmetics_url: String,
    cosmetics_token: String,
    webhook: String,      // for admin server
    main_webhook: String, // for main group
    lb_webhook: String,
    event_queue: Arc<Mutex<event_queue::EventQueue>>,
}

const API_KEY: &str = "B2XwN6Zdt3aRLDhzWq5vVnTgQCEMxkyfJusjrGKe7P49pYmS8b";
async fn verify_api_key(request: Request, next: Next) -> Response {
    let err_response = Response::builder().status(400).body(Body::empty()).unwrap();

    let uri = request.uri();
    if uri.path() == "/" {
        return next.run(request).await;
    }

    let headers = request.headers();
    if let Some(key) = headers.get("api-key") {
        if key.to_str().unwrap() != API_KEY {
            return err_response;
        }
    } else {
        return err_response;
    }

    next.run(request).await
}

pub async fn get_db_conn(url: String, token: String) -> anyhow::Result<Connection> {
    let db = Builder::new_remote(url, token).build().await?;
    Ok(db.connect()?)
}

async fn get_cosmetics(
    State(state): State<AppState>,
    Path(user_id): Path<u64>,
) -> Json<CosmeticUserInfo> {
    println!("getting cosmetics for {user_id}");
    let conn = get_db_conn(state.cosmetics_url, state.cosmetics_token)
        .await
        .unwrap();

    let cosmetic_info = cosmetics::get_cosmetics(user_id, conn)
        .await
        .unwrap_or(CosmeticUserInfo::new(user_id));

    println!("got cosmetics {cosmetic_info:?}");
    Json(cosmetic_info)
}

async fn update_cosmetics(
    State(state): State<AppState>,
    Json(cosmetics): Json<cosmetics::CosmeticUserInfo>,
) -> StatusCode {
    let conn = get_db_conn(state.cosmetics_url, state.cosmetics_token)
        .await
        .unwrap();

    if cosmetics::update_cosmetics(cosmetics, conn).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

async fn get_progress(
    State(state): State<AppState>,
    Path(user_id): Path<u64>,
) -> Json<Option<Progress>> {
    println!("Retrieving Militarum progress for {user_id}");
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let mili_rank_id = match roblox::get_rank_in_group(roblox::MILITARUM_GROUP_ID, user_id).await {
        Ok(None) => {
            println!("Profile {user_id} retrieval failed, not in Militarum");
            return Json(None);
        }
        Ok(Some((id, _))) => id,
        // this error is probably a timeout, we can normally ignore it
        Err(_e) => 999,
    };

    let mut progress = database::get_progress(user_id, mili_rank_id, &conn).await;
    progress.try_update_username().await;
    Json(Some(progress))
}

async fn set_pathway(
    State(state): State<AppState>,
    Path((user_id, pathway)): Path<(u64, String)>,
) -> StatusCode {
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    let mili_rank_id = match roblox::get_rank_in_group(roblox::MILITARUM_GROUP_ID, user_id).await {
        Ok(None) => {
            println!("Profile {user_id} retrieval failed, not in Militarum");
            return StatusCode::BAD_REQUEST;
        }
        Ok(Some((id, _))) => id,
        // this error is probably a timeout, we can normally ignore it
        Err(_e) => 999,
    };

    let mut progress = database::get_progress(user_id, mili_rank_id, &conn).await;
    if pathway == "HELIOS" {
        progress.pathway = Some(Pathway::Helios {
            lead_rts: 0,
            lead_dts: 0,
            helios_lectures: 0,
            co_lead: if mili_rank_id == 4 { Some(0) } else { None },
        })
    } else {
        progress.pathway = None;
    }

    if let Err(e) = database::update_progress(progress, Arc::new(conn)).await {
        eprintln!(
            "Failed to update militarum progress {}, with error {}",
            user_id, e
        );
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
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
        if sol_rank_id != 999 && profile.try_reset_events() {
            update = true;
        }
        if sol_rank_id != 999 && profile.try_update_username().await {
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
    Path(user_id): Path<u64>,
    Json(body): Json<IncEventBody>,
) -> StatusCode {
    let conn = get_db_conn(state.url, state.token).await.unwrap();

    match roblox::get_rank_in_group(roblox::SOL_GROUP_ID, user_id).await {
        Ok(None) => match roblox::get_rank_in_group(roblox::MILITARUM_GROUP_ID, user_id).await {
            Ok(None) | Err(_) => {
                println!("Profile {user_id} retrieval failed, not in SOL");
                return StatusCode::NOT_FOUND;
            }
            Ok(Some((mili_rank_id, _))) => {
                let mut progress = database::get_progress(user_id, mili_rank_id, &conn).await;
                if progress.rank_id != mili_rank_id {
                    progress.reset();
                    progress.rank_id = mili_rank_id;
                }

                progress.try_update_username().await;

                let kind = EventKind::from(body.event_kind);
                match kind {
                    EventKind::RT => progress.rts += 1,
                    EventKind::DT => progress.dts += 1,
                    EventKind::RAID | EventKind::DEFENSE | EventKind::SCRIM => {
                        progress.warfare_events += 1
                    }
                    _ => {}
                }

                if let Err(e) = database::update_progress(progress, Arc::new(conn)).await {
                    eprintln!(
                        "Failed to update militarum progress {}, with error {}",
                        user_id, e
                    );
                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
            }
        },
        Ok(Some((sol_rank_id, _))) => {
            let (mut profile, in_db) = database::get_profile(user_id, sol_rank_id, &conn).await;

            profile.try_reset_events();
            if sol_rank_id != 999 {
                profile.try_update_rank(sol_rank_id);
            }

            profile.events_attended_this_week += body.inc;
            let event_date: DateTime<Utc> = Utc::now();
            profile.last_event_attended_date = Some(event_date);

            profile.try_award_mark();

            if let Err(e) = database::update_profile(profile, in_db, conn.into()).await {
                eprintln!("Failed to update profile {}, with error {}", user_id, e);
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        Err(_e) => return StatusCode::NOT_FOUND,
    };

    StatusCode::OK
}

async fn add_mark(State(state): State<AppState>, Path(user_id): Path<u64>) -> StatusCode {
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
    profile.marks_at_current_rank += 1;
    profile.total_marks += 1;

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

async fn lb(State(state): State<AppState>) -> StatusCode {
    println!("updating lb");
    match stats::weekly_activity_lb(state.lb_webhook, state.url, state.token).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            println!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn default() -> String {
    "hello!".to_string()
}

#[tokio::main]
async fn main() {
    let secrets = fs::read_to_string("Secrets.toml").expect("Secrets.toml does not exist");
    let secrets_table = secrets.parse::<Table>().unwrap();

    let db_token_string = secrets_table.get("DB_TOKEN").unwrap().to_string();
    let db_url_string = secrets_table.get("DB_URL").unwrap().to_string();
    let cos_db_token_string = secrets_table.get("COS_DB_TOKEN").unwrap().to_string();
    let cos_db_url_string = secrets_table.get("COS_DB_URL").unwrap().to_string();
    let webhook_string = secrets_table.get("EVENT_WEBHOOK").unwrap().to_string();
    let main_webhook_string = secrets_table.get("MAIN_EVENT_WEBHOOK").unwrap().to_string();
    let lb_webhook_string = secrets_table.get("LB_WEBHOOK").unwrap().to_string();

    let db_token = util::strip_token(db_token_string);
    let db_url = util::strip_token(db_url_string);
    let cos_db_token = util::strip_token(cos_db_token_string);
    let cos_db_url = util::strip_token(cos_db_url_string);
    let event_webhook = util::strip_token(webhook_string);
    let main_event_webhook = util::strip_token(main_webhook_string);
    let lb_event_webhook = util::strip_token(lb_webhook_string);

    let event_queue = Arc::new(Mutex::new(event_queue::EventQueue::new()));
    let state = AppState {
        token: db_token.clone(),
        url: db_url.clone(),
        cosmetics_token: cos_db_token,
        cosmetics_url: cos_db_url,
        webhook: event_webhook,
        main_webhook: main_event_webhook,
        lb_webhook: lb_event_webhook,
        event_queue: event_queue.clone(),
    };

    let app = Router::new()
        .route("/progress/:id", get(get_progress))
        .route("/progress/:id/pathway/:pathway", post(set_pathway))
        .route("/profiles/:id", get(get_profile))
        .route("/profiles/promotable", get(get_promotable))
        .route("/profiles/create", post(create_profile))
        .route("/profiles/update", post(update_profiles))
        .route("/profiles/increment/:id", post(increment_events))
        .route("/profiles/marks/:id", post(add_mark))
        .route("/events/:id", get(get_hosted))
        .route("/events", put(put_event))
        .route("/events/attended/:id", get(get_events_attended))
        .route("/events/num-attended/:id", get(get_attended))
        .route("/events/info/:id", get(get_event_info_by_info))
        .route("/cosmetics/:id", get(get_cosmetics))
        .route("/cosmetics", post(update_cosmetics))
        .route("/lb", post(lb))
        .route("/", get(default))
        .layer(from_fn(verify_api_key))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let event_loop = event_queue::queue_loop(event_queue, db_url, db_token);
    println!("SOL Mainframe Listening on 0.0.0.0:3000");
    let _ = tokio::join!(
        axum::serve(listener, app).into_future(),
        event_loop.into_future()
    );
}

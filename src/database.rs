use anyhow::Result;
use libsql::{de, Connection};
use sol_util::mainframe::{Event, Profile};

use std::sync::Arc;

// shared db functions
//
pub async fn get_profile(user_id: u64, sol_rank_id: u64, db: &Connection) -> (Profile, bool) {
    let mut get_profile = db
        .prepare("SELECT * FROM profiles WHERE user_id = ?1")
        .await
        .unwrap();

    let profile_response = get_profile.query_row([user_id]).await;
    match profile_response {
        Ok(profile_row) => (de::from_row::<Profile>(&profile_row).unwrap(), true),
        // errors if no row is returned, which means there is no profile in the db
        // (probably)
        Err(_) => (Profile::new(user_id, sol_rank_id), false),
    }
}

pub async fn get_attended(user_id: u64, db: Connection) -> u64 {
    let mut response = db
        .query(
            r#"SELECT COUNT(*) AS events_attended
         FROM events, json_each(attendance)
         WHERE value = ?1
        "#,
            [user_id],
        )
        .await
        .unwrap();

    let row = response.next().await.unwrap();
    match row {
        Some(r) => r.get::<u64>(0).expect("to be a 0th column"),
        None => 0,
    }
}

pub async fn get_events_attended(user_id: u64, db: Connection) -> Vec<u64> {
    let mut rows = db
        .query(
            r#"SELECT event_id
         FROM events, json_each(attendance)
         WHERE value = ?1
        "#,
            [user_id],
        )
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Ok(Some(r)) = rows.next().await {
        events.push(r.get::<u64>(0).unwrap())
    }

    events
}

pub async fn get_event(event_id: i32, db: Connection) -> Result<Option<Event>> {
    let event_response = db
        .query("SELECT * FROM events WHERE event_id = ?1", [event_id])
        .await?
        .next()
        .await?;

    Ok(event_response.map(|event_row| {
        println!("{event_row:?}");
        Event::from_row(&event_row)
    }))
}

pub async fn update_profile(profile: Profile, in_db: bool, db: Arc<Connection>) -> Result<()> {
    if in_db {
        db.execute(
            r#"UPDATE profiles
            SET
                rank_id = ?1,
                total_marks = ?2,
                marks_at_current_rank = ?3,
                events_attended_this_week = ?4,
                last_event_attended_date = ?5
            WHERE user_id = ?6
            "#,
            (
                profile.rank_id,
                profile.total_marks,
                profile.marks_at_current_rank,
                profile.events_attended_this_week,
                profile.last_event_attended_date.unwrap().to_rfc3339(),
                profile.user_id,
            ),
        )
        .await?;
    } else {
        db.execute(r#"
        INSERT INTO profiles (user_id, rank_id, total_marks, marks_at_current_rank, events_attended_this_week, last_event_attended_date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            (
                profile.user_id,
                profile.rank_id,
                profile.total_marks,
                profile.marks_at_current_rank,
                profile.events_attended_this_week,
                profile.last_event_attended_date.unwrap().to_rfc3339(),
            ))
            .await?;
    }

    Ok(())
}

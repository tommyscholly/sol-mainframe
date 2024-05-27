use anyhow::Result;
use libsql::Connection;
use sol_util::mainframe::{Event, Profile};
use sol_util::roblox;
use tokio::time;

use std::collections::VecDeque;
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
        Ok(profile_row) => (Profile::from_row(&profile_row), true),
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
                match profile.last_event_attended_date {
                    Some(d) => d.to_rfc3339(),
                    None => serde_json::to_string(&profile.last_event_attended_date).unwrap(),
                },
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
                match profile.last_event_attended_date {
                    Some(d) => d.to_rfc3339(),
                    None => serde_json::to_string(&profile.last_event_attended_date).unwrap()
                }
            ))
            .await?;
    }

    Ok(())
}

pub async fn remove_profile(user_id: u64, db: Arc<Connection>) -> Result<()> {
    db.execute("DELETE FROM profiles WHERE user_id = ?1", [user_id])
        .await?;
    Ok(())
}

pub async fn get_promotable(db: Connection) -> Result<Vec<u64>> {
    let mut rows = db
        .query(
            r#"
        SELECT * FROM profiles
        WHERE rank_id = 1 OR rank_id = 2 OR rank_id = 3 OR rank_id = 4 OR rank_id = 5"#,
            (),
        )
        .await?;

    let mut users = Vec::new();
    while let Ok(Some(r)) = rows.next().await {
        let profile = Profile::from_row(&r);
        let rank = sol_util::rank::Rank::from_rank_id(profile.rank_id).unwrap();
        if profile.marks_at_current_rank
            >= rank.required_marks().expect("this shouldnt possibly fail")
        {
            users.push(profile.user_id);
        }
    }

    Ok(users)
}

pub async fn update_all(url: String, token: String) -> Result<()> {
    let db = crate::get_db_conn(url.clone(), token.clone()).await?;
    let mut rows = db
        .query(
            r#"
        SELECT * FROM profiles"#,
            (),
        )
        .await?;

    let mut users = VecDeque::new();
    while let Ok(Some(r)) = rows.next().await {
        let profile = Profile::from_row(&r);
        users.push_back(profile);
    }

    drop(db);

    while let Some(mut user) = users.pop_front() {
        let id_opt = match roblox::get_rank_in_group(roblox::SOL_GROUP_ID, user.user_id).await {
            Ok(id_opt) => id_opt,
            Err(e) => {
                println!("Got error {e}, waiting 5 seconds");
                time::sleep(time::Duration::from_secs(5)).await;
                users.push_back(user);
                continue;
            }
        };

        let db = Arc::new(crate::get_db_conn(url.clone(), token.clone()).await?);
        match id_opt {
            Some((id, _)) => {
                if id != user.rank_id {
                    // think its safe to reset the marks
                    let old_rank_id = user.rank_id;
                    user.try_update_rank(id);
                    user.try_reset_events();
                    println!(
                        "Updating user {}: was {} now {}",
                        user.user_id, old_rank_id, id
                    );
                    update_profile(user, true, db).await?;
                }
            }
            // user isnt in sol anymore, need to remove the profile
            None => {
                println!("User {} is no longer in SOL", user.user_id);
                remove_profile(user.user_id, db).await?;
            }
        }
    }

    Ok(())
}

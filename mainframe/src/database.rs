use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, NaiveDateTime, NaiveTime, Utc};
use libsql::Connection;
use sol_util::mainframe::{Event, Profile, Progress};
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
        Err(_) => (Profile::new(user_id, None, sol_rank_id), false),
    }
}

pub async fn get_progress(user_id: u64, mili_rank_id: u64, db: &Connection) -> Progress {
    let default_progress = Progress {
        user_id,
        rank_id: mili_rank_id,
        username: None,
        dts: 0,
        rts: 0,
        warfare_events: 0,
        zac_mins: 0.0,
        pathway: None,
    };

    let mut response = db
        .query(
            "SELECT * FROM militarum_progress WHERE user_id = ?1",
            [user_id],
        )
        .await
        .unwrap();

    let row = match response.next().await {
        Ok(r) => match r {
            Some(r) => r,
            None => return default_progress,
        },
        Err(_) => return default_progress,
    };

    Progress::from_row(&row)
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

pub async fn update_progress(progress: Progress, db: Arc<Connection>) -> Result<()> {
    let rows_updated = db
        .execute(
            r#"UPDATE militarum_progress
            SET
                rank_id = ?1,
                username = ?2,
                rts = ?3,
                dts = ?4,
                warfare_events = ?5,
                zac_mins = ?6
            WHERE user_id = ?7
            "#,
            (
                progress.rank_id,
                progress.username.clone(),
                progress.rts,
                progress.dts,
                progress.warfare_events,
                progress.zac_mins,
                progress.user_id,
            ),
        )
        .await?;

    if rows_updated == 0 {
        db.execute(r#"INSERT INTO militarum_progress (user_id, rank_id, username, rts, dts, warfare_events, zac_mins)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#, (
                progress.user_id,
                progress.rank_id,
                progress.username,
                progress.rts,
                progress.dts,
                progress.warfare_events,
                progress.zac_mins,
        )).await?;
    }

    Ok(())
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
                last_event_attended_date = ?5,
                username = ?6
            WHERE user_id = ?7
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
                profile.username,
                profile.user_id,
            ),
        )
        .await?;
    } else {
        db.execute(r#"
        INSERT INTO profiles (user_id, rank_id, total_marks, marks_at_current_rank, events_attended_this_week, last_event_attended_date, username)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            (
                profile.user_id,
                profile.rank_id,
                profile.total_marks,
                profile.marks_at_current_rank,
                profile.events_attended_this_week,
                match profile.last_event_attended_date {
                    Some(d) => d.to_rfc3339(),
                    None => serde_json::to_string(&profile.last_event_attended_date).unwrap()
                },
                profile.username
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

pub async fn remove_progress(user_id: u64, db: Arc<Connection>) -> Result<()> {
    db.execute(
        "DELETE FROM militarum_progress WHERE user_id = ?1",
        [user_id],
    )
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

fn get_monday() -> DateTime<Utc> {
    let now = Utc::now().naive_utc();
    let current_weekday = now.weekday().num_days_from_monday();

    // Calculate the number of days to subtract to get to the most recent Monday
    let days_to_monday = Duration::days(current_weekday.into());

    // Subtract the days to get this week's Monday
    let this_weeks_monday_date = now.date() - days_to_monday;

    // Set the time to midnight
    let ndt = NaiveDateTime::new(
        this_weeks_monday_date,
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );
    ndt.and_local_timezone(Utc).unwrap()
}

pub async fn get_top(url: String, token: String, top: u64) -> Result<Vec<(String, i32)>> {
    let dt = get_monday();
    let db = crate::get_db_conn(url.clone(), token.clone()).await?;
    let mut rows = db
        .query(
            r#"SELECT user_id, username, events_attended_this_week
            FROM profiles
            WHERE datetime(last_event_attended_date) >= datetime(?2)
            ORDER BY events_attended_this_week DESC
            LIMIT ?1"#,
            (top, dt.to_rfc3339()),
        )
        .await?;

    let mut users = Vec::with_capacity(top as usize);
    while let Ok(Some(r)) = rows.next().await {
        let user_id = r.get::<u64>(0).unwrap();
        let username = match r.get::<Option<String>>(1)? {
            Some(s) => s,
            None => {
                let info = roblox::get_user_info_from_id(user_id).await?;
                info.name
            }
        };
        let events = r.get::<i32>(2).unwrap();
        users.push((username, events))
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

    while let Some(user) = users.pop_front() {
        let id_opt = match roblox::get_rank_in_group(roblox::SOL_GROUP_ID, user.user_id).await {
            Ok(id_opt) => id_opt,
            Err(e) => {
                println!("Got error {e}, waiting 30 seconds");
                time::sleep(time::Duration::from_secs(30)).await;
                users.push_back(user);
                continue;
            }
        };

        let db = Arc::new(crate::get_db_conn(url.clone(), token.clone()).await?);
        match id_opt {
            Some((_id, _)) => {}
            // user isnt in sol anymore, need to remove the profile
            None => {
                println!(
                    "User {}:{:?} is no longer in SOL",
                    user.user_id, user.username
                );
                remove_profile(user.user_id, db).await?;
            }
        }
    }

    let db = crate::get_db_conn(url.clone(), token.clone()).await?;
    let mut rows = db
        .query(
            r#"
        SELECT * FROM militarum_progress"#,
            (),
        )
        .await?;

    let mut mili = VecDeque::new();
    while let Ok(Some(r)) = rows.next().await {
        let profile = Progress::from_row(&r);
        mili.push_back(profile);
    }

    while let Some(user) = mili.pop_front() {
        let id_opt = match roblox::get_rank_in_group(roblox::MILITARUM_GROUP_ID, user.user_id).await
        {
            Ok(id_opt) => id_opt,
            Err(e) => {
                println!("Got error {e}, waiting 30 seconds");
                time::sleep(time::Duration::from_secs(30)).await;
                mili.push_back(user);
                continue;
            }
        };

        let db = Arc::new(crate::get_db_conn(url.clone(), token.clone()).await?);
        match id_opt {
            Some((_id, _)) => {}
            // user isnt in sol anymore, need to remove the profile
            None => {
                println!(
                    "User {}:{:?} is no longer in the Militarum",
                    user.user_id, user.username
                );
                remove_progress(user.user_id, db).await?;
            }
        }
    }

    Ok(())
}

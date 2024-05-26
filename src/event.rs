use libsql::Connection;
use sol_util::mainframe::Event;

use std::sync::Arc;

use crate::{
    database,
    roblox::{self, SOL_GROUP_ID},
};

pub trait Attendance {
    async fn log_attendance(&self, db: Arc<Connection>);
}

impl Attendance for Event {
    async fn log_attendance(&self, db: Arc<Connection>) {
        let attendance = self.attendance.clone();
        for user_id in attendance {
            let event_date = self.event_date;
            let db_ref = db.clone();
            tokio::spawn(async move {
                let sol_rank_id = match roblox::get_rank_in_group(SOL_GROUP_ID, user_id).await {
                    Ok(None) => {
                        return;
                    }
                    Ok(Some((id, _))) => id,
                    Err(e) => panic!("{}", e.to_string()),
                };

                let (mut profile, in_db) =
                    database::get_profile(user_id, sol_rank_id, &db_ref).await;

                profile.try_reset_events();
                profile.try_update_rank(sol_rank_id);

                profile.last_event_attended_date = Some(event_date);
                profile.events_attended_this_week += 1;

                profile.try_award_mark();

                if let Err(e) = database::update_profile(profile, in_db, db_ref).await {
                    eprintln!("Failed to update profile {}, with error {}", user_id, e);
                }
            });
        }
    }
}

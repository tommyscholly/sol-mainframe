use libsql::Connection;
use sol_util::{
    mainframe::{Event, EventKind},
    roblox::MILITARUM_GROUP_ID,
};

use std::sync::Arc;

use crate::{
    database,
    roblox::{self, SOL_GROUP_ID},
};

pub trait Attendance {
    async fn log_attendance(&self, db: Arc<Connection>) -> Vec<u64>;
}

impl Attendance for Event {
    async fn log_attendance(&self, db: Arc<Connection>) -> Vec<u64> {
        let attendance = self.attendance.clone();
        let mut failures = Vec::new();
        for user_id in attendance {
            let event_date = self.event_date;
            let db_ref = db.clone();
            let rank_opt = match roblox::get_rank_in_group(SOL_GROUP_ID, user_id).await {
                Ok(Some((id, _))) => Some(id),
                Ok(None) => None,
                Err(_) => None,
            };

            match rank_opt {
                Some(sol_rank_id) => {
                    let (mut profile, in_db) =
                        database::get_profile(user_id, sol_rank_id, &db_ref).await;

                    profile.try_reset_events();
                    if sol_rank_id != 0 {
                        profile.try_update_rank(sol_rank_id);
                    }
                    profile.try_update_username().await;

                    profile.last_event_attended_date = Some(event_date);
                    profile.events_attended_this_week += 1;

                    profile.try_award_mark();

                    if let Err(e) = database::update_profile(profile, in_db, db_ref).await {
                        failures.push(user_id);
                        eprintln!("Failed to update profile {}, with error {}", user_id, e);
                    }
                }
                None => {
                    let mili_rank_id =
                        match roblox::get_rank_in_group(MILITARUM_GROUP_ID, user_id).await {
                            Ok(Some((id, _))) => id,
                            _ => 1,
                        };

                    if mili_rank_id >= 6 {
                        continue;
                    }

                    let mut progress = database::get_progress(user_id, mili_rank_id, &db_ref).await;
                    if progress.rank_id != mili_rank_id {
                        progress.reset();
                        progress.rank_id = mili_rank_id;
                    }

                    progress.try_update_username().await;

                    let kind = EventKind::from(self.kind.clone());
                    match kind {
                        EventKind::RT => progress.rts += 1,
                        EventKind::DT => progress.dts += 1,
                        EventKind::RAID | EventKind::DEFENSE | EventKind::SCRIM => {
                            progress.warfare_events += 1
                        }
                        _ => {}
                    }

                    if let Err(e) = database::update_progress(progress, db_ref).await {
                        failures.push(user_id);
                        eprintln!(
                            "Failed to update militarum progress {}, with error {}",
                            user_id, e
                        );
                    }
                }
            }
        }

        failures
    }
}

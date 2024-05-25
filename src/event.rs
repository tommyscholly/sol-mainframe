use chrono::{DateTime, Utc};
use libsql::{Connection, Row};
use serde::{Deserialize, Serialize};

use crate::{
    database,
    roblox::{self, SOL_GROUP_ID},
};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Event {
    pub host: u64,
    pub attendance: Vec<u64>, // List of userids that attended the event, including the host
    pub event_date: DateTime<Utc>,
    pub location: String,
    pub kind: String,
}

impl Event {
    pub fn from_json_body(body: EventJsonBody) -> Self {
        let event_date: DateTime<Utc> = Utc::now();
        Self {
            host: body.host,
            attendance: body.attendance,
            event_date,
            location: body.location,
            kind: body.kind,
        }
    }

    pub fn from_row(row: &Row) -> Self {
        let _event_id = row.get::<u64>(0).unwrap();
        let host = row.get::<u64>(1).unwrap();
        let attendance = serde_json::from_str::<Vec<u64>>(row.get_str(2).unwrap()).unwrap();
        let event_date = chrono::DateTime::parse_from_rfc3339(row.get_str(3).unwrap()).unwrap();
        let location = row.get_str(4).unwrap();
        let kind = row.get_str(5).unwrap();

        Self {
            host,
            attendance,
            event_date: event_date.into(),
            location: location.to_string(),
            kind: kind.to_string(),
        }
    }

    pub async fn log_attendance(&self, db: Connection) {
        for user_id in &self.attendance {
            let sol_rank_id = match roblox::get_rank_in_group(SOL_GROUP_ID, *user_id).await {
                Ok(None) => {
                    continue;
                }
                Ok(Some((id, _))) => id,
                Err(e) => panic!("{}", e.to_string()),
            };

            let (mut profile, in_db) =
                database::get_profile(*user_id, sol_rank_id, db.clone()).await;

            profile.try_reset_events();
            profile.try_update_rank(sol_rank_id);

            profile.last_event_attended_date = Some(self.event_date);
            profile.events_attended_this_week += 1;

            profile.try_award_mark();

            if let Err(e) = database::update_profile(profile, in_db, db.clone()).await {
                eprintln!("Failed to update profile {}, with error {}", user_id, e);
            }
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct EventJsonBody {
    pub host: u64,
    pub attendance: Vec<u64>, // List of userids that attended the event, including the host
    pub location: String,
    pub kind: String,
}

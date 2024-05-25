use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

use crate::rank::{self, Rank};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Profile {
    pub user_id: u64,
    pub rank_id: u64,
    pub last_event_attended_date: Option<DateTime<Utc>>,
    pub total_marks: i32,
    pub marks_at_current_rank: i32,
    pub events_attended_this_week: i32,
}

#[allow(unused)]
impl Profile {
    pub fn new(user_id: u64, rank_id: u64) -> Self {
        Self {
            user_id,
            rank_id,
            last_event_attended_date: None,
            total_marks: 0,
            marks_at_current_rank: 0,
            events_attended_this_week: 0,
        }
    }

    pub fn should_promote(&self) -> bool {
        let rank = Rank::from_rank_id(self.rank_id).unwrap();
        if let Some(marks) = rank.required_marks() {
            return self.marks_at_current_rank == marks;
        }

        false
    }

    pub fn try_award_mark(&mut self) -> bool {
        if self.events_attended_this_week == rank::EVENT_PER_WEEK_FOR_MARK {
            self.total_marks += 1;
            self.marks_at_current_rank += 1;

            return true;
        }

        false
    }

    pub fn try_update_rank(&mut self, current_rank_id: u64) {
        if self.rank_id != current_rank_id {
            self.rank_id = current_rank_id;
            self.marks_at_current_rank = 0;
        }
    }

    pub fn try_reset_events(&mut self) {
        let current_date: DateTime<Utc> = Utc::now();
        let weekday = current_date.date_naive().weekday();
        if let Some(date) = self.last_event_attended_date {
            if weekday == chrono::Weekday::Mon && date.date_naive() != current_date.date_naive() {
                self.events_attended_this_week = 0;
            }
        }
    }
}

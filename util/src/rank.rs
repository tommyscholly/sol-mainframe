#![allow(clippy::expect_fun_call)]
use std::fmt::Display;

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

pub const EVENT_PER_WEEK_FOR_MARK: i32 = 4;

#[derive(FromPrimitive, ToPrimitive)]
pub enum Rank {
    Aspirant = 1,
    Neophyte = 2,
    Marine = 3,
    Vanguard = 4,
    Sergeant = 5,
    Legionnaire = 6,
    RetiredAstartes = 7,
    Primaris = 40,
    Chaplain = 50,
    Lieutenant = 60,
    Saint = 100,
    Centurion = 200,
    Captain = 210,
    Praetor = 215,
    Legate = 220,
    Marshal = 225,
    Primarch = 230,
    Mechanicus = 235,
    Warmaster = 240,
    LordSolar = 255,
}

impl Rank {
    pub fn next(&self) -> Option<Rank> {
        match &self {
            Rank::Aspirant => Some(Rank::Neophyte),
            Rank::Neophyte => Some(Rank::Marine),
            Rank::Marine => Some(Rank::Vanguard),
            Rank::Vanguard => Some(Rank::Sergeant),
            Rank::Sergeant => Some(Rank::Legionnaire),
            Rank::Legionnaire => Some(Rank::Primaris),
            Rank::RetiredAstartes => None,
            Rank::Primaris => Some(Rank::Chaplain),
            Rank::Chaplain => Some(Rank::Lieutenant),
            Rank::Lieutenant => Some(Rank::Centurion),
            Rank::Saint => None,
            Rank::Centurion => Some(Rank::Captain),
            Rank::Captain => Some(Rank::Praetor),
            Rank::Praetor => Some(Rank::Legate),
            Rank::Legate => Some(Rank::Marshal),
            Rank::Marshal => Some(Rank::Primarch),
            Rank::Primarch => Some(Rank::Warmaster),
            Rank::Mechanicus => None,
            Rank::Warmaster => Some(Rank::LordSolar),
            Rank::LordSolar => None,
        }
    }
    pub fn from_rank_id(rank_id: u64) -> Option<Rank> {
        FromPrimitive::from_u64(rank_id)
    }

    pub fn required_marks(&self) -> Option<i32> {
        use Rank::*;

        match self {
            Aspirant => Some(2),
            Neophyte => Some(3),
            Marine => Some(4),
            Vanguard => Some(5),
            Sergeant => Some(6),
            _ => None,
        }
    }

    pub fn is_officer(&self) -> bool {
        let value = Rank::to_u64(self).unwrap();
        value != 100 && value >= 40
    }

    pub fn can_host_spars(&self) -> bool {
        let value = Rank::to_u64(self).unwrap();
        value >= 5
    }

    pub fn is_council(&self) -> bool {
        let value = Rank::to_u64(self).unwrap();
        value >= 220
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_str = match self {
            Rank::Aspirant => "Aspirant",
            Rank::Neophyte => "Neophyte",
            Rank::Marine => "Marine",
            Rank::Vanguard => "Vanguard",
            Rank::Sergeant => "Sergeant",
            Rank::Legionnaire => "Legionnaire",
            Rank::RetiredAstartes => "Retired Astartes",
            Rank::Primaris => "Primaris",
            Rank::Chaplain => "Chaplain",
            Rank::Lieutenant => "Lieutenant",
            Rank::Saint => "Saint",
            Rank::Centurion => "Centurion",
            Rank::Captain => "Captain",
            Rank::Praetor => "Praetor",
            Rank::Legate => "Legate",
            Rank::Marshal => "Marshal",
            Rank::Primarch => "Primarch",
            Rank::Mechanicus => "Mechanicus",
            Rank::Warmaster => "Warmaster",
            Rank::LordSolar => "Lord Solar",
        };

        write!(f, "{}", name_str)
    }
}

pub struct MilitarumReqs {
    pub dts: Option<u64>,
    pub rts: Option<u64>,
    pub warfare_events: Option<u64>,
    pub zac_mins: Option<f64>,
}

#[derive(FromPrimitive, ToPrimitive, Clone)]
pub enum MilitarumRank {
    Enlisted = 1,
    Conscript = 2,
    Trooper = 3,
    SeniorTrooper = 4,
}

impl MilitarumRank {
    pub fn from_rank_id(rank_id: u64) -> Option<MilitarumRank> {
        FromPrimitive::from_u64(rank_id)
    }

    pub fn reqs(&self) -> MilitarumReqs {
        use MilitarumRank::*;
        match *self {
            Enlisted => MilitarumReqs {
                dts: None,
                rts: None,
                warfare_events: None,
                zac_mins: None,
            },
            Conscript => MilitarumReqs {
                dts: Some(4),
                rts: Some(4),
                warfare_events: None,
                zac_mins: None,
            },
            Trooper => MilitarumReqs {
                dts: Some(5),
                rts: Some(5),
                warfare_events: Some(10),
                zac_mins: Some(7.0),
            },
            SeniorTrooper => MilitarumReqs {
                dts: Some(6),
                rts: Some(6),
                warfare_events: Some(12),
                zac_mins: Some(10.0),
            },
        }
    }

    pub fn next_rank_name(&self) -> String {
        use MilitarumRank::*;
        match *self {
            Enlisted => "Conscript".into(),
            Conscript => "Trooper".into(),
            Trooper => "Senior Trooper".into(),
            SeniorTrooper => "Astarte".into(),
        }
    }
}

impl Display for MilitarumRank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use MilitarumRank::*;
        let name_str = match *self {
            Enlisted => "Enlisted",
            Conscript => "Conscript",
            Trooper => "Trooper",
            SeniorTrooper => "Senior Trooper",
        };

        write!(f, "{}", name_str)
    }
}

#![allow(clippy::expect_fun_call)]
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub const EVENT_PER_WEEK_FOR_MARK: i32 = 4;

#[derive(FromPrimitive)]
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
}

impl ToString for Rank {
    fn to_string(&self) -> String {
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

        name_str.to_string()
    }
}

use anyhow::Result;
use libsql::{de, Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct CosmeticUserInfo {
    user_id: u64,
    skins: String, // json blob
    fragments: i64,
    equipped: String,           // json blob
    cards: String,              // json blob
    equipped_card_info: String, // json blob
}

impl CosmeticUserInfo {
    pub fn new(user_id: u64) -> Self {
        Self {
            skins: "{}".into(),
            user_id,
            fragments: 0,
            equipped: "{}".into(),
            cards: "{}".into(),
            equipped_card_info: "{}".into(),
        }
    }

    fn from_row(row: &Row) -> Self {
        // let user_id = row.get::<u64>(0).unwrap();
        de::from_row(row).expect("row should convert nicely into a cosmetic user info")
    }
}

pub async fn get_cosmetics(user_id: u64, conn: Connection) -> Result<CosmeticUserInfo> {
    let mut get_cosmetics = conn
        .prepare("SELECT * FROM users WHERE user_id = ?1 LIMIT 1")
        .await
        .unwrap();

    let cosmetics_response = get_cosmetics.query_row([user_id]).await;
    match cosmetics_response {
        Ok(row) => Ok(CosmeticUserInfo::from_row(&row)),
        // errors if no row is returned, which means there is no cosmetics in the db
        // (probably)
        Err(_) => Ok(CosmeticUserInfo::new(user_id)),
    }
}

pub async fn update_cosmetics(info: CosmeticUserInfo, conn: Connection) -> Result<()> {
    let rows_affected = conn.execute("UPDATE users SET skins = ?, fragments = ?, equipped = ?, cards = ?, equipped_card_info = ? WHERE user_id = ?", (
        info.skins.clone(),
        info.fragments,
        info.equipped.clone(),
        info.cards.clone(),
        info.equipped_card_info.clone(),
        info.user_id
    )).await?;

    if rows_affected == 0 {
        conn.execute(
            "INSERT INTO users VALUES (?, ?, ?, ?, ?, ?)",
            (
                info.user_id,
                info.skins,
                info.fragments,
                info.equipped,
                info.cards,
                info.equipped_card_info,
            ),
        )
        .await?;
    }

    Ok(())
}

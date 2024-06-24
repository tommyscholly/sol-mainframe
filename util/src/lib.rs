pub mod mainframe;
pub mod rank;
pub mod roblox;

use anyhow::Result;

use roblox::get_rank_in_groups;

const GAMES_ID: u64 = 33904831;
const MECH_ID: u64 = 8455302;
const DW_ID: u64 = 7366596;
const HG_ID: u64 = 8085509;
// const IK_ID: u64 = 7366406;
const MILI_ID: u64 = 9138660;

pub async fn get_division_tags(user_id: u64) -> Result<String> {
    let mut divison_tags = String::new();
    let info = get_rank_in_groups(vec![HG_ID, GAMES_ID, MECH_ID, DW_ID, MILI_ID], user_id).await?;
    let hg = &info[0];
    let games = &info[1];
    let mech = &info[2];
    let dw = &info[3];
    let mili = &info[4];

    if let Some((rank_id, _)) = hg {
        // ignore hopeful, mechanicus, and the non-hg ranks
        if *rank_id != 1 && *rank_id != 200 && *rank_id <= 250 {
            // let rank_name = rank_name.replace(' ', "-");
            // divison_tags += &format!("{rank_name} ");
            divison_tags += "Hetaeron ";
        }
    }

    if let Some((rank_id, _)) = games {
        // ignore owner
        if *rank_id != 255 {
            divison_tags += "Champion ";
        }
    }

    if let Some((rank_id, rank_name)) = mech {
        // fab gen, archmagos, magos
        if *rank_id == 100 || *rank_id == 50 || *rank_id == 5 {
            let rank_name = rank_name.replace(' ', "-");
            divison_tags += &format!("{rank_name} ");
        }
    }

    if let Some((rank_id, rank_name)) = dw {
        if *rank_id <= 253 {
            let rank_name = rank_name.replace(' ', "-");
            divison_tags += &format!("{rank_name} ");
        }
    }

    if let Some((rank_id, rank_name)) = mili {
        // lord commi, commi gen
        if *rank_id == 40 || *rank_id == 50 {
            let rank_name = rank_name.replace(' ', "-");
            divison_tags += &format!("{rank_name} ");
        }
    }

    Ok(divison_tags)
}

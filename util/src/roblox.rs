use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const SOL_GROUP_ID: u64 = 2764561;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UsernameResponse {
    description: String,
    created: String,
    is_banned: bool,
    external_app_display_name: Option<String>,
    pub id: u64,
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfo {
    id: u64,
    name: String,
    member_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleInfo {
    id: u64,
    name: String,
    rank: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserGroupInfo {
    group: GroupInfo,
    role: RoleInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupResponse {
    data: Vec<UserGroupInfo>,
}

pub async fn get_user_info_from_id(user_id: u64) -> Result<UsernameResponse, reqwest::Error> {
    let response = reqwest::get(format!("https://users.roblox.com/v1/users/{}", user_id)).await?;
    let username_response = response.json::<UsernameResponse>().await?;

    Ok(username_response)
}

#[allow(unused)]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserIdResponse {
    requested_username: String,
    has_verified_badge: bool,
    pub id: u64,
    name: String,
    display_name: String,
}

#[allow(unused)]
#[derive(Deserialize)]
struct UserIdResponsePayload {
    data: Vec<UserIdResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserIdFromUsernamePayload {
    usernames: Vec<String>,
    exclude_banned_users: bool,
}

pub async fn get_user_ids_from_usernames(
    usernames: Vec<String>,
) -> Result<HashMap<String, Option<u64>>, reqwest::Error> {
    let payload = UserIdFromUsernamePayload {
        usernames: usernames.clone(),
        exclude_banned_users: true,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://users.roblox.com/v1/usernames/users")
        .json(&payload)
        .send()
        .await?;

    let user_id_response_payload = response.json::<UserIdResponsePayload>().await?;

    let mut user_id_response_hash_map: HashMap<String, Option<u64>> = HashMap::new();

    for name in usernames.iter() {
        let name_clone = name.clone().to_string();
        user_id_response_hash_map.insert(name_clone, None);
    }
    for user_id_response in user_id_response_payload.data.iter() {
        user_id_response_hash_map.insert(
            user_id_response.requested_username.to_owned(),
            Some(user_id_response.id),
        );
    }

    Ok(user_id_response_hash_map)
}

pub async fn get_rank_in_group(
    group_id: u64,
    user_id: u64,
) -> Result<Option<(u64, String)>, reqwest::Error> {
    let response = reqwest::get(format!(
        "https://groups.roblox.com/v2/users/{}/groups/roles",
        user_id
    ))
    .await?;
    println!("{}", response.status());

    let group_response = response.json::<GroupResponse>().await?;
    let data = group_response.data;
    let index = data
        .iter()
        .position(|group_info| group_info.group.id == group_id);

    if let Some(i) = index {
        let role = &data.get(i).unwrap().role;
        Ok(Some((role.rank, role.name.to_owned())))
    } else {
        Ok(None)
    }
}

pub async fn get_rank_in_groups(
    group_ids: Vec<u64>,
    user_id: u64,
) -> Result<Vec<Option<(u64, String)>>, reqwest::Error> {
    let response = reqwest::get(format!(
        "https://groups.roblox.com/v2/users/{}/groups/roles",
        user_id
    ))
    .await?;
    let group_response = response.json::<GroupResponse>().await?;
    let data = group_response.data;
    let mut rank_ids = Vec::with_capacity(group_ids.len());
    for id in group_ids {
        let index = data.iter().position(|group_info| group_info.group.id == id);

        if let Some(i) = index {
            let role = &data.get(i).unwrap().role;
            rank_ids.push(Some((role.rank, role.name.to_owned())))
        } else {
            rank_ids.push(None)
        }
    }

    Ok(rank_ids)
}

#[derive(Deserialize, Debug)]
struct Headshot {
    #[serde(rename = "imageUrl")]
    image_url: String,
}

#[derive(Deserialize, Debug)]
struct HeadshotResponse {
    data: Vec<Headshot>,
}

pub async fn get_headshot_url(user_id: u64) -> anyhow::Result<String> {
    let response = reqwest::get(format!("https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={user_id}&size=352x352&format=Png&isCircular=false")).await?;
    let headshot_response = response.json::<HeadshotResponse>().await?;

    match headshot_response.data.first() {
        Some(data) => Ok(data.image_url.to_owned()),
        None => Err(anyhow::Error::msg("no data returned from headshot api")),
    }
}

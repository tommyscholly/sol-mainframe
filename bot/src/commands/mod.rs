use poise::serenity_prelude::{
    ButtonStyle, ChannelId, ComponentInteractionCollector, ComponentInteractionDataKind,
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateMessage, CreateSelectMenu, CreateSelectMenuKind,
    EditMessage, Member, UserId,
};
use poise::{command, CreateReply, Modal};
use sol_util::{mainframe, roblox};
use tokio::task::JoinSet;

use crate::rowifi;
use sol_util::rank::Rank;

use crate::AppContext;
use crate::Context;
use crate::Error;

pub mod career;

const SHOUT_CHANNEL_ID: u64 = 700092013781057617;

fn mark_bar(current: i32, goal: i32) -> String {
    let mut result = String::new();

    for _ in 0..current {
        result += "<:RedCheckmark:1241905952144494642> ";
    }

    for _ in current..goal {
        result += "<:UncheckedBox:1241931751295684678> ";
    }

    result
}

async fn get_roblox_id_from_member(member: u64, token: &str) -> Result<Option<u64>, Error> {
    Ok(Some(
        rowifi::get_user(member, 700090648170070056, token).await?,
    ))
}

#[inline]
fn make_footer() -> CreateEmbedFooter {
    CreateEmbedFooter::new("forged by the mechanicum").icon_url("https://cdn.discordapp.com/attachments/1241592098822815818/1241934298320207923/solarMechanicus.png?ex=664c00f2&is=664aaf72&hm=8872db83cd0c47376ff7a89248d962faadad81cdd0714386e60f1a878fa0bbe5&")
}

#[derive(poise::Modal, Debug)]
#[name = "Event Logging Form"]
struct EventInputForm {
    #[name = "Attendees"]
    #[placeholder = "Please input roblox usernames, comma separated"]
    #[min_length = 4]
    usernames: String,
    #[name = "Hosting Location"]
    #[min_length = 4]
    location: String,
}

#[derive(poise::Modal, Debug)]
#[name = "Event Logging Form"]
struct CollectAttendanceForm {
    #[name = "Excluded"]
    #[placeholder = "Please input roblox usernames, comma separated"]
    #[min_length = 4]
    excluded: String,
}

async fn get_rank_from_member(member: &Member, token: &str) -> Result<u64, Error> {
    let roblox_user_id = match get_roblox_id_from_member(member.user.id.get(), token).await? {
        Some(id) => id,
        None => {
            return Err("no user id".into());
        }
    };
    match roblox::get_rank_in_group(roblox::SOL_GROUP_ID, roblox_user_id).await {
        Ok(None) => Err("no rank id".into()),
        Ok(Some((id, _))) => Ok(id),
        Err(_) => Err("no rank id".into()),
    }
}

async fn is_officer(ctx: Context<'_>, member: &Member) -> Result<bool, Error> {
    let sol_rank_id = get_rank_from_member(member, &ctx.data().rowifi_token).await?;
    let rank = Rank::from_rank_id(sol_rank_id).unwrap();
    Ok(rank.is_officer())
}

async fn can_host_spars(ctx: Context<'_>, member: &Member) -> Result<bool, Error> {
    let sol_rank_id = get_rank_from_member(member, &ctx.data().rowifi_token).await?;
    let rank = Rank::from_rank_id(sol_rank_id).unwrap();
    Ok(rank.can_host_spars())
}

#[command(slash_command)]
pub async fn collect_attendance(
    ctx: AppContext<'_>,
    #[choices("DT", "RT", "RAID", "DEFENSE", "SCRIM", "TRAINING", "OTHER")] event_kind: &str,
    location: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let member = ctx.author_member().await.unwrap();
    match is_officer(ctx.into(), &member).await {
        Ok(true) => {}
        Ok(false) | Err(_) => {
            ctx.reply("Only officers can collect attendance.").await?;
            return Ok(());
        }
    }

    let embed = CreateEmbed::new()
        .title(format!(
            "A {event_kind} hosted at {location} is collecting attendance!"
        ))
        .description(
            "Click the 'Attended' button at the bottom. There is a 5 minute timer, so act quickly.",
        )
        .footer(make_footer());

    let button_id = ctx.id();
    let attended_button = CreateButton::new(format!("{button_id}_attended"))
        .label("ATTENDED")
        .style(ButtonStyle::Danger);

    let submit_button = CreateButton::new(format!("{button_id}_submit"))
        .label("SUBMIT")
        .style(ButtonStyle::Primary);

    let filter_button = CreateButton::new(format!("{button_id}_filter"))
        .label("FILTER")
        .style(ButtonStyle::Secondary);

    let action_row = CreateActionRow::Buttons(vec![attended_button, submit_button, filter_button]);
    let reply = poise::CreateReply::default()
        .embed(embed.clone())
        .components(vec![action_row]);

    let reply_handle = ctx.send(reply).await?;

    let mut attended: Vec<u64> = Vec::new();
    let mut submitted = false;
    while let Some(mci) = ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(300))
        .filter(move |mci| {
            mci.data.custom_id == format!("{button_id}_attended")
                || mci.data.custom_id == format!("{button_id}_submit")
                || mci.data.custom_id == format!("{button_id}_filter")
                || mci.data.custom_id == format!("{button_id}_select_menu")
        })
        .await
    {
        if mci.data.custom_id == format!("{button_id}_submit") {
            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                .await?;
            if mci.user.id != member.user.id {
                continue;
            }

            submitted = true;
            break;
        } else if mci.data.custom_id == format!("{button_id}_filter") {
            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                .await?;
            if mci.user.id != member.user.id {
                continue;
            }
            let select_menu = CreateSelectMenu::new(
                format!("{button_id}_select_menu"),
                CreateSelectMenuKind::User {
                    default_users: Some(attended.iter().map(|&id| UserId::new(id)).collect()),
                },
            );
            let follow_up = CreateInteractionResponseFollowup::new()
                .select_menu(select_menu)
                .ephemeral(true);
            mci.create_followup(ctx, follow_up).await?;
        } else if mci.data.custom_id == format!("{button_id}_select_menu") {
            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                .await?;
            if let ComponentInteractionDataKind::UserSelect { values } = &mci.data.kind {
                for user_id in values {
                    if let Some(idx) = attended.iter().position(|&id| id == user_id.get()) {
                        let follow_up = CreateInteractionResponseFollowup::new()
                            .content(format!("Removed <@{}>", user_id.get()))
                            .ephemeral(true);
                        mci.create_followup(ctx, follow_up).await?;
                        attended.remove(idx);
                    }
                }
            }
        } else if mci.data.custom_id == format!("{button_id}_attended") {
            let user_id = mci.user.id.get();
            if let Some(idx) = attended.iter().position(|&id| id == user_id) {
                attended.remove(idx);
            } else {
                attended.push(user_id);
            }

            let desc = attended
                .iter()
                .map(|id| format!("<@{id}>"))
                .collect::<Vec<String>>()
                .join("\n");
            let embed = CreateEmbed::new()
                .title(format!(
                    "A {event_kind} hosted at {location} is collecting attendance!"
                ))
                .description(format!(
                    "Click the 'Attended' button at the bottom. There is a 5 minute timer, so act quickly.\n\nAttendees: {desc}"
                ))
                .footer(make_footer());

            let mut msg = mci.message.clone();
            msg.edit(ctx, EditMessage::new().embed(embed)).await?;

            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                .await?;
        }
    }

    if submitted {
        let mut usernames = Vec::new();
        for id in attended {
            let id = match get_roblox_id_from_member(id, &ctx.data().rowifi_token).await? {
                Some(roblox_id) => roblox_id,
                None => {
                    ctx.say(format!("<@{}> does not have a linked roblox account.", id))
                        .await?;
                    return Ok(());
                }
            };

            let username = roblox::get_user_info_from_id(id).await?.name;
            usernames.push(username);
        }
        let id = member.user.id.get();
        let roblox_user_id = match get_roblox_id_from_member(id, &ctx.data().rowifi_token).await? {
            Some(roblox_id) => roblox_id,
            None => {
                ctx.say(format!("<@{}> does not have a linked roblox account.", id))
                    .await?;
                return Ok(());
            }
        };
        println!("{:?}", usernames);
        mainframe::log_event(roblox_user_id, usernames, location, event_kind.to_string()).await?;

        reply_handle
            .edit(
                ctx.into(),
                CreateReply::default()
                    .content("Event Submitted!")
                    .components(vec![]),
            )
            .await?;
    } else {
        reply_handle
            .edit(
                ctx.into(),
                CreateReply::default()
                    .content("Not Submitted!")
                    .components(vec![]),
            )
            .await?;
    }

    Ok(())
}

#[command(slash_command)]
pub async fn spar(
    ctx: Context<'_>,
    #[description = "Spar place name"] place_name: String,
    #[description = "Access code"] code: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let member = ctx.author_member().await.unwrap();
    match can_host_spars(ctx, &member).await {
        Ok(true) => {}
        Ok(false) | Err(_) => {
            ctx.reply("Only Senior Astartes+ can host spars!").await?;
            return Ok(());
        }
    }
    let chan_id = ChannelId::new(SHOUT_CHANNEL_ID);
    let cache = ctx.serenity_context();
    let chan = chan_id.to_channel(cache).await?;
    let guild_chan = chan.guild().expect("Shout channel should exist");

    let spar_embed = CreateEmbed::new()
        .title("SPAR")
        .field("Host", format!("<@{}>", member.user.id), false)
        .field("Location", place_name, false)
        .field("Access Code", code, false)
        .color(0xF74F00)
        .footer(make_footer());

    let msg = CreateMessage::new().embed(spar_embed);

    guild_chan.send_message(cache, msg).await?;
    ctx.reply("Spar created!").await?;
    Ok(())
}

/// Get event command
#[command(slash_command)]
pub async fn event_info(
    ctx: Context<'_>,
    #[description = "The event ID of interest"] event_id: u64,
) -> Result<(), Error> {
    ctx.defer().await?;
    let event = sol_util::mainframe::get_event(event_id).await?;
    let username = sol_util::roblox::get_user_info_from_id(event.host)
        .await?
        .name;

    let embed = CreateEmbed::new()
        .title(format!("Event {event_id} Info"))
        .footer(make_footer())
        .field("Host", username, true)
        .field(
            "Number of Attendees",
            event.attendance.len().to_string(),
            true,
        )
        .field("Date Hosted", event.event_date.to_string(), true)
        .field("Event Location", event.location, true)
        .field("Event Kind", event.kind, true);

    let reply = poise::CreateReply::default().embed(embed.clone());
    ctx.send(reply).await?;

    Ok(())
}

#[command(slash_command)]
pub async fn add_event(
    ctx: Context<'_>,
    #[description = "User to add an event to"] name: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let member = ctx.author_member().await.unwrap();
    match is_officer(ctx, &member).await {
        Ok(true) => {}
        Ok(false) | Err(_) => {
            ctx.reply("You are not an officer!").await?;
            return Ok(());
        }
    }

    let user_ids = roblox::get_user_ids_from_usernames(&[name.clone()]).await?;
    let user_id = if let Some(Some(id)) = user_ids.get(&name) {
        *id
    } else {
        ctx.say(&format!(
            "No user id for {name}, please check to see if it's their current username."
        ))
        .await?;
        return Ok(());
    };

    sol_util::mainframe::increment_events(user_id, 1).await?;
    ctx.reply(format!("Added an event for {name}")).await?;

    Ok(())
}

#[command(slash_command)]
pub async fn add_mark(
    ctx: Context<'_>,
    #[description = "User to add an event to"] name: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let member = ctx.author_member().await.unwrap();
    match is_officer(ctx, &member).await {
        Ok(true) => {}
        Ok(false) | Err(_) => {
            ctx.reply("You are not an officer!").await?;
            return Ok(());
        }
    }

    let user_ids = roblox::get_user_ids_from_usernames(&[name.clone()]).await?;
    let user_id = if let Some(Some(id)) = user_ids.get(&name) {
        *id
    } else {
        ctx.say(&format!(
            "No user id for {name}, please check to see if it's their current username."
        ))
        .await?;
        return Ok(());
    };

    sol_util::mainframe::add_mark(user_id).await?;
    ctx.reply(format!("Added a mark for {name}")).await?;

    Ok(())
}

#[command(slash_command)]
pub async fn promotable(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let member = ctx.author_member().await.unwrap();
    match is_officer(ctx, &member).await {
        Ok(true) => {}
        Ok(false) | Err(_) => {
            ctx.reply("You are not an officer!").await?;
            return Ok(());
        }
    }

    let promotable_ids = sol_util::mainframe::get_promotable().await?;

    let mut promotable_usernames = Vec::with_capacity(promotable_ids.len());
    let mut set = JoinSet::new();
    for id in promotable_ids {
        set.spawn(async move {
            let user_info = sol_util::roblox::get_user_info_from_id(id).await.unwrap();
            user_info.name
        });
    }

    while let Some(res) = set.join_next().await {
        promotable_usernames.push(res.unwrap());
    }

    let promotable_string = promotable_usernames.join("\n- ");
    ctx.reply(format!("Promotable Astartes:\n- {promotable_string}"))
        .await?;

    Ok(())
}

/// Log event command
#[command(slash_command)]
pub async fn log_event(
    ctx: AppContext<'_>,
    #[choices("DT", "RT", "RAID", "DEFENSE", "SCRIM", "TRAINING", "OTHER")] event_kind: &str,
) -> Result<(), Error> {
    // ctx.defer().await?;
    let member = ctx.author_member().await;
    let roblox_user_id =
        match get_roblox_id_from_member(member.unwrap().user.id.get(), &ctx.data().rowifi_token)
            .await?
        {
            Some(id) => id,
            None => {
                ctx.say("Unable to get author of this command.").await?;
                return Ok(());
            }
        };
    let sol_rank_id = match roblox::get_rank_in_group(roblox::SOL_GROUP_ID, roblox_user_id).await {
        Ok(None) => {
            ctx.say("You are not in SOL").await?;
            return Ok(());
        }
        Ok(Some((id, _))) => id,
        Err(e) => panic!("{}", e.to_string()),
    };
    let rank = Rank::from_rank_id(sol_rank_id).unwrap();
    if !rank.is_officer() {
        ctx.say("You are not an admin").await?;
        return Ok(());
    }

    let data = EventInputForm::execute(ctx).await?;
    if let Some(form) = data {
        let attendees = form
            .usernames
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>();

        mainframe::log_event(
            roblox_user_id,
            attendees,
            form.location.clone(),
            event_kind.to_string(),
        )
        .await?;
        ctx.reply(format!(
            "{} @ {}, hosted by {}",
            event_kind, form.location, roblox_user_id
        ))
        .await?;
    }

    Ok(())
}

#[command(slash_command)]
pub async fn celestine_help(ctx: Context<'_>) -> Result<(), Error> {
    let help_embed = CreateEmbed::new().title("Help").footer(make_footer()).color(0x8888FF).field("/career ?[username] ?[discord user]", "Returns the career of the specified user. Has 2 optional arguments, either a roblox username, a discord user, or nothing at all", true).field("/log_event", "Allows officers to log an event", true).field("/event_info [event_id]", "Returns information about a given event", true).field("/promotable", "Allows officers to see which Astartes are eligible for promotion", true).field("/add_mark [username]", "Adds a mark to the specified user", true);

    let reply = poise::CreateReply::default().embed(help_embed.clone());

    ctx.send(reply).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar() {
        assert_eq!(
            mark_bar(1, 3),
            "<:RedCheckmark:1241905952144494642> <:UncheckedBox:1241931751295684678> <:UncheckedBox:1241931751295684678> "
        )
    }
}

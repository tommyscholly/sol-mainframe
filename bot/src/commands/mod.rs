use poise::serenity_prelude::{
    ButtonStyle, ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed,
    CreateEmbedFooter, CreateInteractionResponse, EditMessage, User,
};
use poise::{command, Modal};
use sol_util::{mainframe, roblox};
use tokio::task::JoinSet;

use crate::rowifi;
use sol_util::get_division_tags;
use sol_util::rank::Rank;

use crate::AppContext;
use crate::Context;
use crate::Error;

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
    #[name = "Event Kind (DT, Raid, Defense, ...)"]
    #[min_length = 2]
    kind: String,
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
    let user_ids = roblox::get_user_ids_from_usernames(vec![name.clone()]).await?;
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
pub async fn promotable(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

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
pub async fn log_event(ctx: AppContext<'_>) -> Result<(), Error> {
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
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        mainframe::log_event(
            roblox_user_id,
            attendees,
            form.location.clone(),
            form.kind.clone(),
        )
        .await?;
        ctx.reply(format!(
            "{} @ {}, hosted by {}",
            form.kind, form.location, roblox_user_id
        ))
        .await?;
    }

    Ok(())
}

/// Default career command
#[command(prefix_command, slash_command)]
pub async fn career(
    ctx: Context<'_>,
    #[description = "Roblox username"] username: Option<String>,
    #[description = "Discord user"] discord_user: Option<User>,
) -> Result<(), Error> {
    let prefix = ctx.prefix();
    if prefix != "/" {
        ctx.reply("Retrieving your profile").await?;
    }

    ctx.defer().await?;
    let roblox_user_id = match username {
        Some(ref name) => {
            let user_ids = roblox::get_user_ids_from_usernames(vec![name.clone()]).await?;
            if let Some(Some(id)) = user_ids.get(name) {
                *id
            } else {
                ctx.say(&format!(
                    "No user id for {name}, please check to see if it's their current username."
                ))
                .await?;
                return Ok(());
            }
        }
        None => {
            let member = match discord_user {
                Some(user) => user.id,
                None => ctx.author_member().await.unwrap().user.id,
            }
            .get();
            match get_roblox_id_from_member(member, &ctx.data().rowifi_token).await? {
                Some(id) => id,
                None => {
                    ctx.say("Unable to get author of this command.").await?;
                    return Ok(());
                }
            }
        }
    };

    let (
        user_profile_result,
        num_events_result,
        user_info_result,
        headshot_result,
        division_tags_result,
        rank_result,
    ) = tokio::join!(
        sol_util::mainframe::get_profile(roblox_user_id),
        sol_util::mainframe::get_num_attendance(roblox_user_id),
        roblox::get_user_info_from_id(roblox_user_id),
        roblox::get_headshot_url(roblox_user_id),
        get_division_tags(roblox_user_id),
        roblox::get_rank_in_group(roblox::SOL_GROUP_ID, roblox_user_id)
    );

    let (sol_rank_id, _) = match rank_result? {
        Some((id, rank_name)) => (id, rank_name),
        None => {
            ctx.say("User is not in SOL.").await?;
            return Ok(());
        }
    };
    let rank = match Rank::from_rank_id(sol_rank_id) {
        Some(r) => r,
        None => {
            ctx.say("User is not in SOL.").await?;
            return Ok(());
        }
    };

    let user_profile = match user_profile_result {
        Ok(p) => p,
        Err(e) => {
            if let Some(name) = username {
                ctx.reply(format!("{name} does not have a profile. {e}"))
                    .await?;
            } else {
                ctx.reply(format!("You do not have a profile. {e}")).await?;
            }
            return Ok(());
        }
    };
    let user_info = user_info_result?;
    let num_events = num_events_result?;
    let headshot_url = match headshot_result {
        Ok(url) => url,
        Err(_) => "".to_string(),
    };
    let divison_tags = match division_tags_result {
        Ok(tags) => tags,
        Err(_) => "".to_string(),
    };

    let next_rank = rank.next();

    let marks = user_profile.total_marks;
    let rank_marks = user_profile.marks_at_current_rank;

    let embed = CreateEmbed::new()
        .title(format!("{}{} {}", divison_tags, rank, user_info.name))
        .field(
            "Total Events Attended",
            format!("{num_events} Events"),
            true,
        )
        .description(format!("**{}** Career Marks", marks))
        .footer(make_footer())
        .thumbnail(headshot_url)
        .color(0x800000);

    let embed = match rank.required_marks() {
        Some(req_marks) => {
            let mark_bar = mark_bar(rank_marks, req_marks);
            embed
                .field(
                    "Weekly Events Attended",
                    format!("**{}**/4 Events", user_profile.events_attended_this_week),
                    true,
                )
                .field(
                    format!("Progress to {}", next_rank.unwrap()),
                    format!("{mark_bar}: {}/{} Marks", rank_marks, req_marks),
                    false,
                )
        }
        None => match next_rank {
            Some(next) => embed.field(
                format!("Progress to {next}"),
                format!("There is no more automatic progression for {rank}"),
                false,
            ),
            None => embed.field(
                "Max Rank Achieved",
                format!("There is no rank above {rank}"),
                false,
            ),
        },
    };

    let button_id = ctx.id();
    let career_button = CreateButton::new(format!("{button_id}_career"))
        .label("CAREER")
        .style(ButtonStyle::Danger);

    let honors_button = CreateButton::new(format!("{button_id}_events"))
        .label("EVENTS")
        .style(ButtonStyle::Secondary);

    let action_row = CreateActionRow::Buttons(vec![career_button, honors_button]);

    let reply = poise::CreateReply::default()
        .embed(embed.clone())
        .components(vec![action_row]);

    ctx.send(reply).await?;

    let mut cached_events_embed: Option<CreateEmbed> = None;
    while let Some(mci) = ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(30))
        .filter(move |mci| {
            mci.data.custom_id == format!("{button_id}_career")
                || mci.data.custom_id == format!("{button_id}_events")
        })
        .await
    {
        if mci.data.custom_id == format!("{button_id}_events") {
            let e = match cached_events_embed {
                Some(ref e) => e.clone(),
                None => {
                    let events_vec =
                        sol_util::mainframe::get_events_attended(roblox_user_id).await?;
                    let embed = CreateEmbed::new()
                        .title(format!("{}'s attended events", user_info.name))
                        .footer(make_footer())
                        .color(0x800000);

                    let events_string = events_vec
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<String>>()
                        .join(", ");
                    let embed = embed.field("Attended Event Ids", events_string, true);
                    cached_events_embed = Some(embed.clone());
                    embed
                }
            };

            let mut msg = mci.message.clone();
            msg.edit(ctx, EditMessage::new().embed(e)).await?;

            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                .await?;
        } else {
            let mut msg = mci.message.clone();
            msg.edit(ctx, EditMessage::new().embed(embed.clone()))
                .await?;

            mci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                .await?;
        }
    }

    Ok(())
}

#[command(slash_command)]
pub async fn celestine_help(ctx: Context<'_>) -> Result<(), Error> {
    let help_embed = CreateEmbed::new().title("Help").footer(make_footer()).color(0x8888FF).field("/career ?[username] ?[discord user]", "Returns the career of the specified user. Has 2 optional arguments, either a roblox username, a discord user, or nothing at all", true).field("/log_event", "Allows officers to log an event", true).field("/event_info [event_id]", "Returns information about a given event", true).field("/promotable", "Allows officers to see which Astartes are eligible for promotion", true);

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

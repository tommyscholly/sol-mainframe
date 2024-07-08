use poise::command;
use poise::serenity_prelude::{
    ButtonStyle, ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed,
    CreateInteractionResponse, EditMessage, User,
};
use sol_util::rank::MilitarumRank;

use crate::commands::{get_roblox_id_from_member, make_footer, mark_bar};
use sol_util::{get_division_tags, rank::Rank, roblox};

use crate::Context;
use crate::Error;

#[command(slash_command)]
pub async fn progress(
    ctx: Context<'_>,
    #[description = "Roblox username"] username: Option<String>,
    #[description = "Discord user"] discord_user: Option<User>,
) -> Result<(), Error> {
    let roblox_user_id = match username {
        Some(ref name) => {
            let user_ids = roblox::get_user_ids_from_usernames(&[name.clone()]).await?;
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

    let progress = sol_util::mainframe::get_progress(roblox_user_id).await?;
    let mili_rank = match MilitarumRank::from_rank_id(progress.rank_id) {
        Some(rank) => rank,
        None => {
            ctx.reply("Not a Militarum member!").await?;
            return Ok(());
        }
    };
    let primary_group_id = roblox::get_primary_group_id(roblox_user_id).await?;
    let primaried = primary_group_id == roblox::MILITARUM_GROUP_ID;

    let headshot_result = roblox::get_headshot_url(roblox_user_id).await;
    let headshot_url = headshot_result.unwrap_or("".to_string());
    let embed = CreateEmbed::new()
        .title(format!(
            "{} {}",
            mili_rank,
            progress.username.unwrap_or(format!("{roblox_user_id}"))
        ))
        .field(
            "Militarum Primaried",
            if primaried {
                "<:RedCheckmark:1241905952144494642>"
            } else {
                "<:UncheckedBox:1241931751295684678>"
            },
            true,
        )
        .footer(make_footer())
        .thumbnail(headshot_url)
        .color(0x568259);

    let reqs = mili_rank.reqs();
    let embed = match reqs.dts {
        Some(dts) => {
            let dt_bar = mark_bar(progress.dts.try_into().unwrap(), dts.try_into().unwrap());
            embed.field("Defense Trainings", dt_bar, false)
        }
        None => embed,
    };
    let embed = match reqs.rts {
        Some(rts) => {
            let rt_bar = mark_bar(progress.rts.try_into().unwrap(), rts.try_into().unwrap());
            embed.field("Raid Trainings", rt_bar, false)
        }
        None => embed,
    };
    let embed = match reqs.warfare_events {
        Some(we) => {
            let we_bar = mark_bar(
                progress.warfare_events.try_into().unwrap(),
                we.try_into().unwrap(),
            );
            embed.field("Warfare Events", we_bar, false)
        }
        None => embed,
    };
    let embed = match reqs.zac_mins {
        Some(mins) => {
            if progress.zac_mins >= mins {
                embed.field(
                    format!("{} Minutes ZAC", mins),
                    "<:RedCheckmark:1241905952144494642>",
                    true,
                )
            } else {
                embed.field(
                    format!("{} Minutes ZAC", mins),
                    "<:UncheckedBox:1241931751295684678>",
                    true,
                )
            }
        }
        None => embed,
    };

    let reply = poise::CreateReply::default().embed(embed);

    ctx.send(reply).await?;
    Ok(())
}

#[command(slash_command)]
pub async fn career(
    ctx: Context<'_>,
    #[description = "Roblox username"] username: Option<String>,
    #[description = "Discord user"] discord_user: Option<User>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let roblox_user_id = match username {
        Some(ref name) => {
            let user_ids = roblox::get_user_ids_from_usernames(&[name.clone()]).await?;
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
        // user_info_result,
        headshot_result,
        division_tags_result,
        // rank_result,
    ) = tokio::join!(
        sol_util::mainframe::get_profile(roblox_user_id),
        sol_util::mainframe::get_num_attendance(roblox_user_id),
        // roblox::get_user_info_from_id(roblox_user_id),
        roblox::get_headshot_url(roblox_user_id),
        get_division_tags(roblox_user_id),
        // roblox::get_rank_in_group(roblox::SOL_GROUP_ID, roblox_user_id)
    );

    // let (sol_rank_id, _) = match rank_result? {
    //     Some((id, rank_name)) => (id, rank_name),
    //     None => {
    //         ctx.say("User is not in SOL.").await?;
    //         return Ok(());
    //     }
    // };

    let user_profile = match user_profile_result {
        Ok(p) => p,
        Err(_e) => {
            if let Some(name) = username {
                ctx.reply(format!("{name} does not have a profile."))
                    .await?;
            } else {
                ctx.reply("You do not have a profile.").await?;
            }
            return Ok(());
        }
    };

    let rank = match Rank::from_rank_id(user_profile.rank_id) {
        Some(r) => r,
        None => {
            ctx.say("User is not in SOL.").await?;
            return Ok(());
        }
    };

    // let user_info = user_info_result?;
    let num_events = num_events_result.unwrap_or(0);
    let headshot_url = headshot_result.unwrap_or("".to_string());
    let divison_tags = division_tags_result.unwrap_or("".to_string());

    let next_rank = rank.next();

    let marks = user_profile.total_marks;
    let rank_marks = user_profile.marks_at_current_rank;
    let username = user_profile
        .username
        .unwrap_or(format!("User {}", user_profile.user_id));

    let embed = CreateEmbed::new()
        .title(format!("{}{} {}", divison_tags, rank, username))
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
                        .title(format!("{}'s attended events", username))
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

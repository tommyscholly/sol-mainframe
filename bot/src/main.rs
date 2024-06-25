mod commands;
mod rowifi;

use poise::serenity_prelude::{self as serenity, ActivityData, GuildId};
use toml::Table;

use std::fs::read_to_string;

struct Data {
    rowifi_token: String,
} // Poise user data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
type AppContext<'a> = poise::ApplicationContext<'a, Data, Error>;

fn strip_token(token: String) -> String {
    token
        .strip_prefix('"')
        .unwrap()
        .strip_suffix('"')
        .unwrap()
        .to_string()
}

#[tokio::main]
async fn main() {
    let secrets = read_to_string("Secrets.toml").expect("Secrets.toml does not exist");
    let secrets_table = secrets.parse::<Table>().unwrap();

    let bot_token_string = secrets_table.get("BOT_TOKEN").unwrap().to_string();
    let rowifi_token_string = secrets_table.get("WIFI_TOKEN").unwrap().to_string();

    let bot_token = strip_token(bot_token_string);
    let rowifi_token = strip_token(rowifi_token_string);

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::log_event(),
            commands::career(),
            commands::event_info(),
            commands::add_event(),
            commands::promotable(),
            commands::celestine_help(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".to_string()),
            ..Default::default()
        },
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}!", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                ctx.set_presence(
                    Some(ActivityData::watching("Luetin09")),
                    serenity::OnlineStatus::DoNotDisturb,
                );
                let guild_id = GuildId::new(700090648170070056);
                guild_id.mem
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { rowifi_token })
            })
        })
        .options(options)
        .build();

    let intents =
        serenity::GatewayIntents::privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(bot_token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap()
}

use regex::Regex;
use serenity::{
    model::{
        channel::Message,
        Permissions,
    },
    prelude::*,
    framework::standard::
    {
        CommandResult,
        macros::{
            command,
            group,
        },
    },
    client::bridge::gateway::ShardId,
};
use std::time::Instant;
use crate::{
    PREFIX,
    ShardManagerContainer,
    botmods::utils::BotModule,
};
use lazy_static;

lazy_static!(
    pub static ref MOD_GENERAL: BotModule = BotModule {
        command_group: &GENERAL_GROUP,
        command_pattern: vec![
            Regex::new(format!(r"^{}ping$", PREFIX).as_str()).unwrap(),
            Regex::new(format!(r"^{}about$", PREFIX).as_str()).unwrap(),
            Regex::new(format!(r"^{}invite$", PREFIX).as_str()).unwrap(),
        ],
        editors: vec![],
        interactors: vec![],
        watchers: vec![],
    };
);

#[group]
#[summary = "General commands"]
#[commands(ping, about, invite)]
struct General;

#[command]
#[description = "Simple command to check if the bot is online"]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let bef = Instant::now();

    let mut msg = msg.channel_id.say(&ctx.http, "Pong!").await?;
    
    let t = bef.elapsed().as_millis();
    
    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            msg.edit(ctx, |m| m.content("Error: Couldn't get shard manager")).await?;
            return Ok(());
        },
    };
    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;
    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.edit(ctx, |m| m.content("Error: No shard found")).await?;
            return Ok(());
        },
    }; 

    match runner.latency {
        Some(d) => {
            msg.edit(&ctx, |m| {
                m.content(format!("Ping:    {} ms\nAPI latency:    {} ms", t, d.as_millis()));
                m
            }).await?;
        }
        None => {
            msg.edit(&ctx, |m| {
                m.content(format!("Ping:    {} ms\nAPI latency:    Not available", t));
                m
            }).await?;
        }
    };
    
    Ok(())
}

#[command]
#[description = "About the bot"]
pub async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let curr_user = ctx.cache.current_user().await;
    
    let av = curr_user.face();

    msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title(format!("About {}", curr_user.name));
                    e.description("A Discord bot to compile Latex/AsciiMath snippets ~~and fetch steps from Wolfram|Alpha~~, written in Rust");
                    e.image(av);
                    e.field("Made by", "Nanu#3294", false);
                    e.field("Check out the github repo!", "[Nanu00/wabot-rs](https://github.com/Nanu00/wabot-rs)", false);
                    e
                });
                m
            }).await?;
    Ok(())
}

#[command]
#[description = "Get an invite link!"]
pub async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    let curr_user = ctx.cache.current_user().await;
    match curr_user.invite_url(&ctx.http, Permissions::from_bits(392256).unwrap()).await {
        Ok(url) => {
            msg.channel_id.say(&ctx.http, url).await?
        },
        Err(e) => {
            msg.channel_id.say(&ctx.http, format!("Error generating invite: {}", e)).await?
        },
    };
    Ok(())
}

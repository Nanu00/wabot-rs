use serenity::{
    async_trait, client,
    model::{
        gateway::Ready,
        channel::Message,
    },
    prelude::*,
    framework::standard::macros::{
        group,
        hook,
    },
};

use std::{env, error};

mod botmods;
use botmods::general::*;
use botmods::math::*;

pub struct Handler;

pub fn get_token() -> Result<String, Box<dyn error::Error>> {
    let token = env::var("DISCORD_TOKEN")?;
    client::validate_token(&token)?;
    Ok(token)
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }
    
    async fn message(&self, ctx: Context, msg: Message) {
        inline_latex(&ctx, &msg).await;
    }
}

#[group]
#[commands(ping, about, invite)]
struct General;

#[group]
#[commands(ascii, latex)]
struct Math;

#[hook]
pub async fn unknown_cmd(ctx: &Context, msg: &Message, u_cmd: &str) {
    msg.channel_id.say(&ctx.http, format!("Command `{}` not found", &u_cmd)).await.expect("Unknown error");
}

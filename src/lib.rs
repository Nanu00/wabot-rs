use serenity::{
    async_trait, client,
    model::gateway::Ready,
    prelude::*,
    framework::standard::macros::group,
};

use std::{env, error};

mod botmods;
use botmods::general::*;
use botmods::math::*;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }
}

#[group]
#[commands(ping, about)]
struct General;

#[group]
#[commands(ascii, latex)]
struct Math;

pub fn get_token() -> Result<String, Box<dyn error::Error>> {
    let token = env::var("DISCORD_TOKEN")?;
    client::validate_token(&token)?;
    Ok(token)
}

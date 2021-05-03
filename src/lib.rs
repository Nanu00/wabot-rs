use serenity::{
    async_trait, client,
    model::gateway::Ready,
    prelude::*,
    framework::standard::{
        macros::{group, help, check, hook},
        StandardFramework, CommandGroup
    },
};

use std::{env, process};

mod botmods;
use botmods::general::*;

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

pub fn get_token() -> String {
    let token = match env::var("DISCORD_TOKEN") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error getting the token: {}", e);
            process::exit(1);
        }
    };
    
    match client::validate_token(&token) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error getting the token: {}", e);
            process::exit(1);
        },
    };

    token
}

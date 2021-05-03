use serenity::{
    async_trait, client,
    model::{channel::Message, gateway::Ready},
    prelude::*,
    framework::standard::{
        CommandResult, macros::{command, group, help, check, hook},
        StandardFramework, CommandGroup
    },
};
use std::{
    env, process,
};
use tokio;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }
}

#[group]
#[commands(ping)]
struct General;

#[tokio::main]
async fn main() {
    
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("`"))
        .group(&GENERAL_GROUP);

    let mut bot = match Client::builder(&get_token())
        .event_handler(Handler)
        .framework(framework)
        .await
        {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error creating the client: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = bot.start().await {
        eprintln!("Client error: {}", e);
    }
}

fn get_token() -> String {
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

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    println!("Ponging");
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

use serenity::{
    async_trait, client,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::{
    env, process,
};
use tokio;

struct Handler;

#[async_trait]
impl EventHandler for Handler {

}

#[tokio::main]
async fn main() {
    let mut bot = match Client::builder(&get_token()).event_handler(Handler).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error creating the client: {}", e);
            process::exit(1);
        }
    };
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

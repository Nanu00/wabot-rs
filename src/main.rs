use serenity::{
    prelude::*,
    framework::standard::StandardFramework
};

use std::process;

use tokio;

use wabot::*;

#[tokio::main]
async fn main() {
    
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("`"))
        .group(&GENERAL_GROUP);

    let token = match get_token() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error getting the token: {}", e);
            process::exit(1);
        }
    };

    let mut bot = match Client::builder(&token)
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

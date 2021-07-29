use serenity::{
    prelude::*,
    framework::standard::StandardFramework,
    client::bridge::gateway::GatewayIntents,
};
use std::{
    process,
};
use wabot::{unknown_cmd, Handler, GENERAL_GROUP, HELP, MATH_GROUP, PREFIX, CONFIG, WOLFRAM_GROUP, load_queues};

#[tokio::main]
async fn main() {
    
    let token = {CONFIG.read().await.get::<String>("discord_token").unwrap()};

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(PREFIX))
        .group(&GENERAL_GROUP)
        .group(&MATH_GROUP)
        .group(&WOLFRAM_GROUP)
        .help(&HELP)
        .unrecognised_command(unknown_cmd);

    let mut bot = match Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .intents(GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES)
        .await
        {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error creating the client: {}", e);
            process::exit(1);
        }
    };
    
    load_queues(&bot).await;

    if let Err(e) = bot.start_autosharded().await {
        eprintln!("Client error: {}", e);
    }
}
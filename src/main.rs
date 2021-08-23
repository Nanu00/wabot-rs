use serenity::{
    prelude::*,
    framework::standard::StandardFramework,
    client::bridge::gateway::GatewayIntents,
};
use std::process;
use wabot::{
    unknown_cmd,
    Handler,
    // HELP,
    PREFIX,
    CONFIG,
    load_queues,
    botmods::MODS
};

#[tokio::main]
async fn main() {
    
    let token = {CONFIG.read().await.get::<String>("discord_token").unwrap()};
    let application_id = {CONFIG.read().await.get::<u64>("discord_appid").unwrap()};

    let mut framework = StandardFramework::new()
        .configure(|c| c.prefix(PREFIX))
        // .help(&HELP)
        .unrecognised_command(unknown_cmd);

    for m in MODS.iter() {
        framework.group_add(m.command_group);
    }

    let mut bot = match Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .application_id(application_id)
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

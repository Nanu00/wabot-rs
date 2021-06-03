use serenity::{
    prelude::*,
    framework::standard::StandardFramework,
};
use std::{
    process,
    sync::Arc,
};
use wabot::{get_token, unknown_cmd, Handler, GENERAL_GROUP, HELP, MATH_GROUP};
use wabot::ShardManagerContainer;

pub static PREFIX: &str = "---";

// pub struct ShardManagerContainer;

// impl TypeMapKey for ShardManagerContainer {
//     type Value = Arc<Mutex<ShardManager>>;
// }

#[tokio::main]
async fn main() {
    
    let framework = StandardFramework::new()
        .configure(|c| c.prefix(PREFIX))
        .group(&GENERAL_GROUP)
        .group(&MATH_GROUP)
        .help(&HELP)
        .unrecognised_command(unknown_cmd);

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
    
    {
        let mut data = bot.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&bot.shard_manager));
    }

    if let Err(e) = bot.start_autosharded().await {
        eprintln!("Client error: {}", e);
    }
}
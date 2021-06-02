use serenity::{
    prelude::*,
    framework::standard::StandardFramework,
};
use std::{process, time::Duration};
use wabot::*;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("^"))
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
    
    let manager = bot.shard_manager.clone();

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(30)).await;

            let lock = manager.lock().await;
            let shard_runners = lock.runners.lock().await;

            for (id, runner) in shard_runners.iter() {
                println!(
                    "Shard ID {} is {} with a latency of {:?}",
                    id,
                    runner.stage,
                    runner.latency,
                );
            }
        }
    });

    if let Err(e) = bot.start_autosharded().await {
        eprintln!("Client error: {}", e);
    }
}
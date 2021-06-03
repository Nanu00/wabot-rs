use serenity::{
    model::{
        channel::Message,
        Permissions,
    },
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command
    }
};

#[command]
#[description = "Simple command to check if the bot is online"]
pub async fn ping(ctx: &Context, msg: &Message, ) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[command]
#[description = "About the bot"]
pub async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let curr_user = ctx.cache.current_user().await;
    
    let av = curr_user.face();

    msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title(format!("About {}", curr_user.name));
                    e.description("A Discord bot to compile Latex/AsciiMath snippets ~~and fetch steps from Wolfram|Alpha~~, written in Rust");
                    e.image(av);
                    e.field("Made by", "Nanu#3294", false);
                    e.field("Check out the github repo!", "[Nanu00/wabot-rs](https://github.com/Nanu00/wabot-rs)", false);
                    e
                });
                m
            }).await?;
    Ok(())
}

#[command]
#[description = "Get an invite link!"]
pub async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    let curr_user = ctx.cache.current_user().await;
    match curr_user.invite_url(&ctx.http, Permissions::from_bits(392256).unwrap()).await {
        Ok(url) => {
            msg.channel_id.say(&ctx.http, url).await?
        },
        Err(e) => {
            msg.channel_id.say(&ctx.http, format!("Error generating invite: {}", e)).await?
        },
    };
    Ok(())
}
use serenity::{
    model::channel::Message, 
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command
    }
};

#[command]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[command]
pub async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title("About wabot");
                    e.description("A Discord bot to compile Latex/AsciiMath snippets~~ and fetch steps from Wolfram|Alpha~~, written in Rust");
                    e.image("https://cdn.discordapp.com/avatars/787599246136311848/a8d00cc4e9d36e2babaef362172f7085.png?size=128");
                    e.field("Made by", "Nanu#3294", false);
                    e.field("Check out the github repo!", "[Nanu00/wabot-rs](https://github.com/Nanu00/wabot-rs)", false);
                    e
                });
                m
            }).await?;
    Ok(())
}

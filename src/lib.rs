use serenity::{
    async_trait, client,
    model::{
        gateway::Ready,
        channel::Message,
        id::UserId,
    },
    prelude::*,
    framework::standard::{
        macros::{
            group,
            hook,
            help,
        },
        help_commands,
        HelpOptions,
        CommandGroup,
        CommandResult,
        Args,
    }
};

use std::{
    env, error,
    collections::{
        HashSet,
    },
};

mod botmods;
use botmods::general::*;
use botmods::math::*;

pub struct Handler;

pub fn get_token() -> Result<String, Box<dyn error::Error>> {
    let token = env::var("DISCORD_TOKEN")?;
    client::validate_token(&token)?;
    Ok(token)
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }
    
    async fn message(&self, ctx: Context, msg: Message) {
        inline_latex(&ctx, &msg).await;
    }
}

#[group]
#[summary = "General commands"]
#[commands(ping, about, invite)]
struct General;

#[group]
#[summary = "Math formatting commands"]
#[commands(ascii, latex)]
struct Math;

#[hook]
pub async fn unknown_cmd(ctx: &Context, msg: &Message, u_cmd: &str) {
    msg.channel_id.say(&ctx.http, format!("Command `{}` not found", &u_cmd)).await.expect("Unknown error");
}

#[help]
#[individual_command_tip = "Here is a list of available commands.\nPass a command as an argument to help to know more."]
#[command_not_found_text = "Could not find: `{}`."]
#[max_levenshtein_distance(3)]
#[indention_prefix = "+"]
#[lacking_permissions = "Hide"]
#[lacking_role = "Nothing"]
#[wrong_channel = "Strike"]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}
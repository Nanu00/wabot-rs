use serenity::{
    async_trait,
    model::{
        gateway::{
            Ready,
            Activity,
        },
        channel::Message,
        id::UserId,
        event::MessageUpdateEvent,
        prelude::Interaction
    },
    prelude::{Client, Context, EventHandler, RwLock, TypeMapKey},
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
    },
    client::bridge::gateway::ShardManager,
};
use tokio::sync::Mutex;
use std::{
    collections::{
        HashSet,
        VecDeque,
    },
    sync::Arc,
};
use config::Config;
#[macro_use]
extern crate lazy_static;

pub mod botmods;
use botmods::general::*;
use botmods::markup::*;
use botmods::wolfram::*;

pub static PREFIX: &str = "---";

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

lazy_static!{
    pub static ref CONFIG: RwLock<Config> = RwLock::new(Config::default().merge(config::File::with_name("config")).unwrap().clone());
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
        ctx.set_activity(Activity::listening(format!("{}help", PREFIX))).await;
    }
    
    async fn message(&self, ctx: Context, msg: Message) {
        match inline_latex(&ctx, &msg).await {
            Ok(_) => {return},
            Err(_) => {return}
        } //TODO: Error handle
    }
    
    async fn message_update(&self, ctx: Context, _: Option<Message>, _: Option<Message>, upd_event: MessageUpdateEvent) {
        if upd_event.content.is_some() {
            botmods::utils::edit_handler(&ctx, &upd_event).await;
        }
    }
    
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        botmods::utils::component_interaction_handler(&ctx, interaction).await;
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

#[group]
#[summary = "Wolfram commands"]
#[commands(wolfram)]
struct Wolfram;

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

pub async fn load_queues(c: &Client) {
    let mut data = c.data.write().await;
    data.insert::<ShardManagerContainer>(Arc::clone(&c.shard_manager));
    data.insert::<MathMessages>(Arc::new(RwLock::new(VecDeque::with_capacity(botmods::markup::EDIT_BUFFER_SIZE))));
    data.insert::<WolframMessages>(Arc::new(RwLock::new(VecDeque::with_capacity(botmods::wolfram::EDIT_BUFFER_SIZE))));
}
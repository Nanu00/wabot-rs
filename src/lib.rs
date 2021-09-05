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
    pin::Pin,
    fs::File,
};
use futures::{
    Future,
    future::join_all
};
use ron::de::from_reader;
use serde::Deserialize;
#[macro_use]
extern crate lazy_static;

pub mod botmods;
use botmods::utils::{
    Editable,
    Interactable,
};

lazy_static!{
    pub static ref CONFIG_DIR: String = format!("{}/.config/wally", env!("HOME"));
    pub static ref CONFIG: Config = load_config();
    pub static ref PREFIX: String = CONFIG.prefix.clone();
}
pub static EDIT_BUFFER_SIZE: usize = 10;
pub static INTERACT_BUFFER_SIZE: usize = 10;

#[derive(Deserialize)]
pub struct Config {
    pub discord_token: String,
    pub discord_appid: u64,
    prefix: String
}

fn load_config() -> Config {
    let path = format!("{}/config.ron", CONFIG_DIR.as_str());
    let f = File::open(&path).expect("Failed reading config file!");
    let config: Config = match from_reader(f) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Failed parsing config file:\n{}", e);
            std::process::exit(1);
        }
    };
    return config;
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct Editables;

impl TypeMapKey for Editables {
    type Value = Arc<RwLock<VecDeque<Box<dyn Editable + Send + Sync>>>>;
}

pub struct Interactables;

impl TypeMapKey for Interactables {
    type Value = Arc<RwLock<VecDeque<Box<dyn Interactable + Send + Sync>>>>;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
        ctx.set_activity(Activity::listening(format!("{}help", PREFIX.as_str()))).await;
    }
    
    async fn message(&self, ctx: Context, msg: Message) {
        let mut watcher_futures: Vec<Pin<Box<dyn Future<Output = CommandResult> + Send>>> = vec![];
        for m in botmods::MODS.iter() {
            for watcher in &m.watchers {
                watcher_futures.push(watcher(ctx.clone(), msg.clone()));
            }
        }
        join_all(watcher_futures).await;
    }
    
    async fn message_update(&self, ctx: Context, _: Option<Message>, _: Option<Message>, upd_event: MessageUpdateEvent) {
        if upd_event.author.is_some() && upd_event.author.as_ref().unwrap().bot {
            return
        }

        let editables_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<Editables>().expect("Oops!").clone() //TODO: Error handling
        };

        {
            let mut editables = editables_lock.write().await;
            editables.make_contiguous();
            
            for i in editables.iter_mut() {
                if i.get_input_message_id() == upd_event.id {
                    i.edit(&ctx).await.unwrap();
                    return
                }
            }
        }

        let mut editor_futures: Vec<Pin<Box<dyn Future<Output = ()> + Send>>> = vec![];
        for m in botmods::MODS.iter() {
            for editor in &m.editors {
                editor_futures.push(editor(ctx.clone(), upd_event.clone()));
            }
        }
        join_all(editor_futures).await;
    }
    
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let interactables_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<Interactables>().expect("Oops!").clone() //TODO: Error handling
        };

        {
            let mut interactables = interactables_lock.write().await;
            interactables.make_contiguous();
            
            for i in interactables.iter_mut() {
                for j in i.get_response_message_id() {
                    if interaction.clone().message_component().is_some() && interaction.clone().message_component().unwrap().message.regular().is_some() && interaction.clone().message_component().unwrap().message.regular().unwrap().id == j {
                        i.interaction_respond(&ctx, interaction.clone()).await.unwrap();
                    }
                }
            }
        }

        let mut interactor_futures: Vec<Pin<Box<dyn Future<Output = ()> + Send>>> = vec![];
        for m in botmods::MODS.iter() {
            for interactor in &m.interactors {
                interactor_futures.push(interactor(ctx.clone(), interaction.clone()));
            }
        }
        join_all(interactor_futures).await;
    }
}

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
    data.insert::<Editables>(Arc::new(RwLock::new(VecDeque::with_capacity(EDIT_BUFFER_SIZE))));
    data.insert::<Interactables>(Arc::new(RwLock::new(VecDeque::with_capacity(INTERACT_BUFFER_SIZE))));
}

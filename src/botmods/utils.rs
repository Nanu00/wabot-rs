use serenity::{
    async_trait,
    builder::{
        CreateButton,
        CreateSelectMenuOption,
        CreateActionRow,
        CreateComponents
    },
    model::{
        prelude::{
            MessageUpdateEvent,
            Interaction,
            ReactionType,
        },
        channel::Message,
        interactions::message_component::ButtonStyle,
        id::MessageId,
    },
    prelude::{
        Context,
        SerenityError
    },
    framework::standard::{
        CommandGroup,
        CommandResult,
    },
};
use std::pin::Pin;
use futures::Future;
use regex::Regex;
use crate::{
    Interactables,
    Editables,
    EDIT_BUFFER_SIZE,
    INTERACT_BUFFER_SIZE
};

pub struct BotModule {
    pub command_group: &'static CommandGroup,
    pub command_pattern: Vec<Regex>,
    pub editors: Vec<fn(Context, MessageUpdateEvent) -> Pin<Box<dyn Future<Output = ()> + Send>>>,
    pub interactors: Vec<fn(Context, Interaction) -> Pin<Box<dyn Future<Output = ()> + Send>>>,
    pub watchers: Vec<fn(Context, Message) -> Pin<Box<dyn Future<Output = CommandResult> + Send>>>
}

#[async_trait]
pub trait Editable {
    async fn edit(&mut self, ctx: &Context) -> Result<(), crate::botmods::errors::Error>;
    fn get_response_message_id(&self) -> Vec<MessageId>;
    fn get_input_message_id(&self) -> MessageId;
    fn get_command_pattern(&self) -> Regex;
}

#[async_trait]
pub trait Interactable {
    async fn interaction_respond(&mut self, ctx: &Context, interaction: Interaction) -> Result<(), crate::botmods::errors::Error>;
    fn get_response_message_id(&self) -> Vec<MessageId>;
}

pub async fn push_to_interactables(ctx: &Context, i: Box<dyn Interactable + Send + Sync>) {
    let interactables_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<Interactables>().expect("Oops!").clone() //TODO: Error handling
    };
    
    {
        let mut interactables = interactables_lock.write().await;
        interactables.push_front(i);
        
        if interactables.len() > INTERACT_BUFFER_SIZE {
            interactables.truncate(INTERACT_BUFFER_SIZE);
        }
    }
}

pub async fn push_to_editables(ctx: &Context, i: Box<dyn Editable + Send + Sync>) {
    let editables_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<Editables>().expect("Oops!").clone() //TODO: Error handling
    };
    
    {
        let mut editables = editables_lock.write().await;
        editables.push_front(i);
        
        if editables.len() > EDIT_BUFFER_SIZE {
            editables.truncate(EDIT_BUFFER_SIZE);
        }
    }
}

#[derive(Debug)]
pub enum Buttons {
    Delete,
    Next,
    Prev,
    Pod(String, usize),
    Invalid     // Not for actual use
}

impl ToString for Buttons {
    fn to_string(&self) -> String {
        match &self {
            Buttons::Delete => "Delete".to_string(),
            Buttons::Next => "Next".to_string(),
            Buttons::Prev => "Previous".to_string(),
            Buttons::Pod(s, _) => {
                let mut s = s.to_string();
                if s.len() > 20 {
                    s.truncate(17);
                    s.push_str("...");
                }
                s.to_string()
            }
            Buttons::Invalid => "".to_string(),
        }
    }
}

impl From<&str> for Buttons {
    fn from(s: &str) -> Buttons {
        let i = Regex::new(r"^POD([[:digit:]]+)").unwrap();
        if let Some(c) = i.captures(s) {
            if let Some(m) = c.get(1) {
                if let Ok(n) = m.as_str().parse::<usize>() {
                    return Buttons::Pod("".to_string(), n)
                }
            }
        }
        match s {
            "DEL" => Buttons::Delete,
            "NEX" => Buttons::Next,
            "PRE" => Buttons::Prev,
            _ => Buttons::Invalid,
        }
    }
}

impl Buttons {
    fn to_id_string(&self) -> String {
        match &self {
            Buttons::Delete => "DEL".to_string(),
            Buttons::Next => "NEX".to_string(),
            Buttons::Prev => "PRE".to_string(),
            Buttons::Pod(_, n) => format!("POD{}", n),
            Buttons::Invalid => "".to_string(),
        }
    }

    fn to_emoji(&self) -> ReactionType {
        match &self {
            Buttons::Delete => ReactionType::Unicode("🗑️".to_string()),
            Buttons::Next => ReactionType::Unicode("\u{27a1}".to_string()),
            Buttons::Prev => ReactionType::Unicode("\u{2b05}".to_string()),
            Buttons::Pod(_, _) => ReactionType::Unicode("\u{1f48a}".to_string()),
            Buttons::Invalid => ReactionType::Unicode("\u{1f6ab}".to_string()),
        }
    }

    pub fn to_button(&self) -> impl FnOnce(&mut CreateButton) -> &mut CreateButton + '_ {
        let label = self.to_string();
        let id = self.to_id_string();
        let emoji = self.to_emoji();
        |b| {
            b.style(ButtonStyle::Primary);
            b.label(label);
            b.custom_id(id);
            b.emoji(emoji);
            b.disabled(false);
            b
        }
    }

    pub fn add_buttons<'b>(a: &'b mut CreateComponents, vb: Vec<Buttons>) -> &'b mut CreateComponents {
        let mut ib = vb.into_iter();
        
        let mut rows: Vec<CreateActionRow> = vec![];
        
        while ib.len() > 0 {
            rows.push(CreateActionRow::default());
            let i = rows.len();
            for _ in 0..3 {
                if let Some(b) = ib.next() {
                    rows[i-1].create_button(b.to_button());
                } else {
                    continue;
                }
            }
        }

        a.set_action_rows(rows);
        a
    }
}

#[derive(Debug)]
pub struct MenuItem {
    label: String,
    emoji: Option<ReactionType>,
    description: String,
    value: String,
}

impl MenuItem {
    pub fn new(label: String, emoji: Option<ReactionType>, value: String, description: String) -> MenuItem {
        MenuItem {
            label,
            emoji,
            value,
            description,
        }
    }
    
    pub fn to_csmop(&self) -> CreateSelectMenuOption {
        let mut i = CreateSelectMenuOption::default();
        i.label(self.label.clone());
        i.value(self.value.clone());
        if let Some(e) = self.emoji.clone() {
            i.emoji(e);
        }
        i.default_selection(false);
        i.description(self.description.clone());
        i
    }

    pub fn add_menu<'b>(c: &'b mut CreateComponents, vmi: Vec<MenuItem>, custom_id: &str) -> &'b mut CreateComponents {
        c.create_action_row( |a| {
            a.create_select_menu( |sm| {
                sm.options( |smops| {
                    for i in vmi {
                        smops.add_option(i.to_csmop());
                    }
                    smops
                });
                sm.custom_id(custom_id);
                sm
            });
            a
        });
        c
    }
}

pub async fn loading_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId) -> Result<Message, SerenityError> {
    c_id.send_message(&ctx.http, |m| {
        m.content("Doing stuff <a:loading:840650882286223371>");
        m
    }).await
}

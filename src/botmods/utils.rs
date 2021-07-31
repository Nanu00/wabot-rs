use serenity::{
    model::{
        prelude::{
            MessageUpdateEvent,
            Interaction,
            ReactionType,
        },
        channel::Message,
        interactions::{
            ButtonStyle,
            InteractionResponseType,
            InteractionMessage,
            InteractionApplicationCommandCallbackDataFlags,
        },
    },
    builder::{
        CreateButton,
        CreateMessage,
        CreateActionRow,
    },
    prelude::{Context, SerenityError},
};
use std::cmp::PartialEq;
use regex::Regex;
use crate::{
    botmods::{
        markup,
        wolfram
    },
    WolframMessages,
    MathMessages
};

pub async fn loading_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId) -> Result<Message, SerenityError> {
    c_id.send_message(&ctx.http, |m| {
        m.content("Doing stuff <a:loading:840650882286223371>");
        m
    }).await
}

pub async fn edit_handler(ctx: &Context, msg_upd_event: &MessageUpdateEvent) {
    let new_content = match &msg_upd_event.content {
        Some(c) => String::from(c),
        None => {return}
    };
    
    for (r, ct) in markup::EDITMATCH.iter() {
        if let Some(m) = r.captures(&new_content) {
            if let Some(n) = m.name("i") {
                markup::edit_handler(ctx, msg_upd_event, n.as_str(), ct).await;
            }
        }
    }

    for (r, ct) in wolfram::EDITMATCH.iter() {
        if let Some(m) = r.captures(&new_content) {
            if let Some(n) = m.name("i") {
                wolfram::edit_handler(ctx, msg_upd_event, n.as_str(), ct).await;
            }
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

impl PartialEq<String> for Buttons {
    fn eq(&self, other: &String) -> bool {
        &self.to_id_string() == other
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
}

pub async fn component_interaction_handler(ctx: &Context, interaction: Interaction) {
    
    let message = match interaction.message.as_ref().unwrap() {
        InteractionMessage::Regular(m) => m,
        _ => {return}
    };
    
    let mut is_wolf = false;
    let mut is_mkup = false;

    {
        let (wms_lock, mms_lock) = {
            let data_read = ctx.data.read().await;
            (data_read.get::<WolframMessages>().expect("Oops!").clone(), data_read.get::<MathMessages>().expect("Oops!").clone())  //TODO: Error handling
        };

        {
            let mut wms = wms_lock.write().await;
            wms.make_contiguous();
            
            for i in wms.iter() {
                if i.header_message.is_some() && i.header_message.as_ref().unwrap().id == message.id {
                    is_wolf = true;
                } else {
                    for j in i.pod_messages.iter() {
                        if j.message.is_some() && j.message.as_ref().unwrap().id == message.id {
                            is_wolf = true;
                        }
                    }
                }
            }
        }

        {
            let mut mms = mms_lock.write().await;
            mms.make_contiguous();
            
            for i in mms.iter() {
                if i.message.is_some() && i.message.as_ref().unwrap().id == message.id {
                    is_mkup = true;
                }
            }
        }
    }
    
    if is_wolf {
        wolfram::component_interaction_handler(ctx, interaction.clone()).await;
        interaction.create_interaction_response(ctx, |r|{
            r.kind(InteractionResponseType::UpdateMessage);
            r.interaction_response_data(|d|{
                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                d
            });
            r
        }).await.unwrap();
    } else if is_mkup {
        markup::component_interaction_handler(ctx, interaction.clone()).await;
        interaction.create_interaction_response(ctx, |r|{
            r.kind(InteractionResponseType::UpdateMessage);
            r.interaction_response_data(|d|{
                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                d
            });
            r
        }).await.unwrap();
    }
}

pub fn add_components<'b, 'a>(m: &'b mut CreateMessage<'a>, vb: Vec<Buttons>) -> &'b mut CreateMessage<'a> {
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

    m.components( |c| {
        c.set_action_rows(rows);
        c
    });
    m
}
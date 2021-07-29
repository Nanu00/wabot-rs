use serenity::{
    model::{
        prelude::MessageUpdateEvent,
        channel::Message,
    },
    prelude::*,
};
use crate::botmods::{
    markup,
    wolfram
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
    
    for (r, ct) in markup::REGMATCH.iter() {
        if let Some(m) = r.captures(&new_content) {
            if let Some(n) = m.name("i") {
                markup::edit_handler(ctx, msg_upd_event, n.as_str(), ct).await;
            }
        }
    }

    for (r, ct) in wolfram::REGMATCH.iter() {
        if let Some(m) = r.captures(&new_content) {
            if let Some(n) = m.name("i") {
                wolfram::edit_handler(ctx, msg_upd_event, n.as_str(), ct).await;
            }
        }
    }
}
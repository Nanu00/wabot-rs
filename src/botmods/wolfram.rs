use regex::Regex;
use std::{
    fmt::Display,
    fmt,
    sync::Arc,
    collections::VecDeque,
};
use crate::{
    botmods::{
        errors,
        utils::{
            loading_msg,
            Buttons,
            MenuItem,
        },
    },
    CONFIG,
    PREFIX
};
use serenity::{framework::standard::
    {
        CommandResult, macros::command, Args,
    },
    model::{
        channel::Message,
        id::ChannelId,
        interactions::message_component::{
            ComponentType,
            InteractionMessage,
            MessageComponentInteraction,
        },
        prelude::MessageUpdateEvent,
    },
    prelude::{
        Context,
        TypeMapKey,
        RwLock,
    }
};
use serde_json::Value;
use urlencoding::encode;



pub const EDIT_BUFFER_SIZE: usize = 10;

#[derive(PartialEq)]
pub enum CmdType {
    Wolfram
}

lazy_static!{
    pub static ref EDITMATCH: Vec<(Regex, CmdType)> = vec![
        (Regex::new(format!(r"^{}wolfram (?P<i>.*)$", PREFIX).as_str()).unwrap(), CmdType::Wolfram),
        (Regex::new(format!(r"^{}w (?P<i>.*)$", PREFIX).as_str()).unwrap(), CmdType::Wolfram),
    ];
}

#[derive(Debug)]
pub struct WolframMessages;

impl TypeMapKey for WolframMessages {
    type Value = Arc<RwLock<VecDeque<WolfMessage>>>;
}

async fn wolf_messages_pusher(ctx: &Context, wm: WolfMessage) {
    let wms_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<WolframMessages>().expect("Oops!").clone()  //TODO: Error handling
    };

    {
        let mut wms = wms_lock.write().await;
        wms.push_front(wm);

        if wms.len() > EDIT_BUFFER_SIZE {
            wms.truncate(EDIT_BUFFER_SIZE);
        }
    }
}

pub async fn edit_handler(ctx: &Context, msg_upd_event: &MessageUpdateEvent, arg: &str, _: &CmdType) {
    let lm = loading_msg(&ctx, &msg_upd_event.channel_id).await.unwrap();
    let inp_message = msg_upd_event.channel_id.message(&ctx, msg_upd_event.id).await.unwrap();
    
    let opts = vec![
        Opt::Format("image".to_string()),
        Opt::Output("json".to_string()),
        ];

    let new_w = QueryResult::new(Opt::Input(arg.to_string()), opts).await.unwrap();
    let mut new_wm = WolfMessage::new(new_w.clone(), inp_message.clone(), new_w.pods).await;
    
    let wms_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<WolframMessages>().expect("Oops!").clone()  //TODO: Error handling
    };
    
    {
        let mut wms = wms_lock.write().await;
        wms.make_contiguous();
        
        let mut msg_index: Option<usize> = None;

        for (i, j) in wms.iter().enumerate() {
            if j.inp_message.id == inp_message.id {
                msg_index = Some(i);
            }
        }
        
        let msg_index = match msg_index {
            Some(i) => i,
            None => {return}
        };
        
        let old_wm = wms.get_mut(msg_index).unwrap();
        old_wm.delete(ctx).await;
        
        lm.delete(ctx).await.unwrap();
        new_wm.send_messages(ctx).await;
        
        wms.insert(msg_index, new_wm);
        wms.remove(msg_index+1);
    }
    
}

pub async fn component_interaction_handler(ctx: &Context, interaction: MessageComponentInteraction) {
    let message = match interaction.message {
        InteractionMessage::Regular(m) => m,
        _ => {return}
    };
    
    let user = match interaction.member {
        Some(u) => u.user,
        None => interaction.user,
    };

    let c = interaction.data;

    match c.component_type {
        ComponentType::Button => {
            let wms_lock = {
                let data_read = ctx.data.read().await;
                data_read.get::<WolframMessages>().expect("Oops!").clone()  //TODO: Error handling
            };

            {
                let mut wms = wms_lock.write().await;
                wms.make_contiguous();
                
                for i in wms.iter_mut() {
                    if i.inp_message.author != user {
                        continue;
                    }
                    if i.header_message.as_ref().unwrap().id == message.id {
                        match Buttons::from(c.custom_id.as_str()) {
                            Buttons::Delete => {i.delete(ctx).await;}
                            // Buttons::Pod(_, n) => {i.pod_messages[n].send_message(ctx, message.channel_id).await.unwrap();},
                            _ => {}
                        }
                    } else {
                        for j in i.pod_messages.iter_mut() {
                            if j.message.is_some() && j.message.as_ref().unwrap().id == message.id {
                                match Buttons::from(c.custom_id.as_str()) {
                                    Buttons::Next => {
                                        if j.curr_spod == (j.pod.subpods.len()-1) {
                                            j.change_spod(ctx, 0).await.unwrap();
                                        } else {
                                            j.change_spod(ctx, j.curr_spod+1).await.unwrap();
                                        }
                                    },
                                    Buttons::Prev => {
                                        if j.curr_spod == 0 {
                                            j.change_spod(ctx, j.pod.subpods.len() -1).await.unwrap();
                                        } else {
                                            j.change_spod(ctx, j.curr_spod-1).await.unwrap();
                                        }
                                    },
                                    Buttons::Delete => {
                                        j.delete_message(ctx).await.unwrap();
                                    },
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                
            }
        },
        ComponentType::SelectMenu => {
            let wms_lock = {
                let data_read = ctx.data.read().await;
                data_read.get::<WolframMessages>().expect("Oops!").clone()  //TODO: Error handling
            };

            {
                let mut wms = wms_lock.write().await;
                wms.make_contiguous();
                
                for i in wms.iter_mut() {
                    if i.inp_message.author != user {
                        continue;
                    }
                    if i.header_message.as_ref().unwrap().id == message.id {
                        for v in c.values.iter() {
                            let pod_re = Regex::new(r"^POD(?P<n>\d+)").unwrap();
                            if let Some(cap) = pod_re.captures(v) {
                                if let Some(n) = cap.name("n") {
                                    let n = n.as_str().parse::<usize>().unwrap();
                                    i.pod_messages[n].send_message(ctx, message.channel_id).await.unwrap();
                                }
                            }
                        }
                    }
                }
            }
        },
        _ => {}
    }

}

#[derive(Clone, Debug)]
enum Opt {
    // Podstate(String),
    Output(String),
    Input(String),
    Format(String),
}

impl Display for Opt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Opt::Podstate(s) => {write!(f, "podstate={}", encode(s))},
            Opt::Output(s) => {write!(f, "output={}", encode(s))},
            Opt::Input(s) => {write!(f, "input={}", encode(s))},
            Opt::Format(s) => {write!(f, "format={}", encode(s))},
        }
    }
}

#[derive(Clone, Debug)]
pub struct QueryResult {
    input: Opt,
    pods: Vec<Pod>,
    error: bool,
    json: Value,
    options: Vec<Opt>
}

impl QueryResult {
    async fn new(input: Opt, options: Vec<Opt>) -> Result<QueryResult, errors::Error> {
        let appid = {CONFIG.read().await.get::<String>("w_appid").unwrap()};

        let mut url = format!("https://api.wolframalpha.com/v2/query?appid={}&{}", &appid, input);
        for i in options.iter() {
            url = format!("{}&{}", url, i);
        }

        let result = reqwest::get(url).await?
            .json::<serde_json::Value>().await?;
        
        let error = result["queryresult"]["error"].as_bool().unwrap();
        
        let mut pods = vec![];
        
        if let Some(ps) = result["queryresult"]["pods"].as_array() {
            for i in ps.iter() {
                pods.push(Pod::new(i).await);
            }
        }

        Ok(QueryResult{
            input,
            pods,
            error,
            json: result["queryresult"].clone(),
            options
        })
    }
}

#[derive(Clone, Debug)]
pub struct Pod {
    title: String,
    subpods: Vec<Subpod>,
    json: Value
}

impl Pod {
    async fn new(json: &Value) -> Pod {
        let mut subpods = vec![];
        
        if let Some(sps) = json["subpods"].as_array() {
            for i in sps.iter() {
                subpods.push(Subpod::new(i).await);
            }
        }

        Pod {
            title: json["title"].as_str().unwrap().to_string(),
            subpods,
            json: json.clone()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Subpod {
    title: String,
    image: Image,
    json: Value
}

impl Subpod {
    async fn new(json: &Value) -> Subpod {
        Subpod {
            title: json["title"].as_str().unwrap().to_string(),
            image: Image::new(&json["img"]).await,
            json: json.clone()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Image {
    src: String,
    title: String,
    alt: String,
    img_type: String,
    json: Value,
}

impl Image {
    async fn new(json: &Value) -> Image {
        Image {
            src: json["src"].as_str().unwrap().to_string(),
            alt: json["alt"].as_str().unwrap().to_string(),
            title: json["title"].as_str().unwrap().to_string(),
            img_type: json["type"].as_str().unwrap().to_string(),
            json: json.clone(),
        }
    }
}

#[derive(Debug)]
pub struct WolfMessage {
    result: QueryResult,
    inp_message: Message,
    pub header_message: Option<Message>,
    pub pod_messages: Vec<PodMessage>,
}

impl WolfMessage {
    async fn new(r: QueryResult, inp: Message, pods: Vec<Pod>) -> WolfMessage {
        let mut pod_messages = vec![];

        for i in pods.iter() {
            pod_messages.push(
                PodMessage::new(i).await
            );
        }
        
        WolfMessage{
            result: r,
            inp_message: inp,
            header_message: None,
            pod_messages
        }
    }
    
    async fn send_messages(&mut self, ctx: &Context) {
        
        let buttons = vec![
            Buttons::Delete,
        ];
        
        let mut m_items: Vec<MenuItem> = vec![];

        for (i, j) in self.pod_messages.iter().enumerate() {
            m_items.push(
                MenuItem::new(j.pod.title.clone(), None, format!("POD{}", i), format!("Pod {}", i+1))
            )
        }
        
        self.header_message = Some(self.inp_message.channel_id.send_message(&ctx.http, |m|{
            m.embed(|e| {
                e.title("Wolfram query");
                e.description("Results provided by [Wolfram|Alpha](https://www.wolframalpha.com/)");
                if let Opt::Input(s) = &self.result.input {
                    e.field("Input", s, false);
                }
                e.footer(|f| {
                    f.icon_url(self.inp_message.author.avatar_url().unwrap());
                    f.text(format!("Requested by {}#{}", self.inp_message.author.name, self.inp_message.author.discriminator));
                    f
                });
                e
            });
            m.components(|c| {
                MenuItem::add_menu(c, m_items, "POD");
                Buttons::add_buttons(c, buttons);
                c
            });
            m
        }).await.unwrap());
        
    } //TODO: Error handling
    
    async fn delete(&mut self, ctx: &Context) {
        self.header_message.as_ref().unwrap().delete(ctx).await.unwrap();

        for i in self.pod_messages.iter_mut() {
            if i.message.is_some() {
                i.message.as_ref().unwrap().delete(ctx).await.unwrap();
            }
        }
    } //TODO: Error handling
}

#[derive(Debug)]
pub struct PodMessage {
    pod: Pod,
    curr_spod: usize,
    pub message: Option<Message>,
}

impl PodMessage {
    async fn new(pod: &Pod) -> PodMessage {
        PodMessage {
            pod: pod.clone(),
            curr_spod: 0,
            message: None
        }
    }

    async fn send_message(&mut self, ctx: &Context, channel_id: ChannelId) -> Result<(), errors::Error> {
        if let Some(_) = &self.message {
            return Ok(())
        }
        let mut buttons: Vec<Buttons> = vec![
            Buttons::Delete,
        ];
        if self.pod.subpods.len() > 1 {
            buttons.extend(vec![
                Buttons::Prev,
                Buttons::Next,
            ]);
        }

        let mut buttons = buttons.into_iter();

        self.message = Some(channel_id.send_message(&ctx.http, |m|{
            m.embed( |e| {
                e.title(&self.pod.title);
                e.image(&self.pod.subpods[0].image.src);
                e
            });
            m.components(|c| {
                c.create_action_row(|a| {
                    for _ in 0..buttons.len() {
                        a.create_button(buttons.next().unwrap().to_button());
                    }
                    a
                })
            });
            m
        }).await.unwrap());

        Ok(())
    }
    
    async fn delete_message(&mut self, ctx: &Context) -> Result<(), errors::Error> {
        if let Some(m) = self.message.as_ref() {
            m.delete(ctx).await.unwrap();
            self.message = None;
        }
        Ok(())
    }
    
    async fn change_spod(&mut self, ctx: &Context, spod: usize) -> Result<(), errors::Error> {
        if let Some(mut sm) = self.message.clone() {
            sm.edit(&ctx.http, |m| {
                m.embed( |e| {
                    e.title(&self.pod.title);
                    e.image(&self.pod.subpods[spod].image.src);
                    e
                });
                m
            }).await.unwrap();
            self.curr_spod = spod;
        }
        Ok(())
    }
}

#[command]
#[aliases("w")]
pub async fn wolfram(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {
    let lm = loading_msg(&ctx, &msg.channel_id).await?;
    let query = match arg.remains() {
        Some(r) => Ok(r),
        None => {
            let err = errors::Error::ArgError(1, 0);
            errors::err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &err).await?;
            Err(err)
        }
    }?;
    
    let opts = vec![
        Opt::Format("image".to_string()),
        Opt::Output("json".to_string()),
        ];
    
    let w = QueryResult::new(Opt::Input(query.to_string()), opts).await.unwrap();
    
    let mut wm = WolfMessage::new(w.clone(), msg.clone(), w.pods).await;

    lm.delete(&ctx.http).await?;
    
    wm.send_messages(ctx).await;

    wolf_messages_pusher(ctx, wm).await;
    
    Ok(())
}

use regex::Regex;
use std::{
    fmt::Display,
    fmt,
    pin::Pin,
};
use futures::Future;
use crate::{
    botmods::{
        errors,
        utils::{
            loading_msg,
            Buttons,
            MenuItem,
            BotModule,
            Editable,
            Interactable,
            push_to_editables,
            push_to_interactables,
        },
    },
    CONFIG,
    PREFIX,
    Interactables,
    Editables
};
use serenity::{
    async_trait,
    framework::standard::{
        CommandResult,
        macros::{
            command,
            group
        },
        Args,
    },
    model::{
        channel::Message,
        id::{
            ChannelId,
            MessageId,
        },
        interactions::{
            message_component::{
                ComponentType,
                InteractionMessage,
            },
            InteractionResponseType,
            InteractionApplicationCommandCallbackDataFlags,
            Interaction,
        },
        prelude::MessageUpdateEvent,
    },
    prelude::Context,
};
use serde_json::Value;
use urlencoding::encode;
use serde::{
    Serialize,
    Deserialize
};
use lazy_static;

lazy_static!(
    pub static ref MOD_WOLFRAM: BotModule = BotModule {
        command_group: &WOLFRAM_GROUP,
        command_pattern: vec![
            Regex::new(format!(r"^{}wolfram .*$", PREFIX).as_str()).unwrap(),
            Regex::new(format!(r"^{}w .*$", PREFIX).as_str()).unwrap(),
        ],
        editors: vec![
            edit_handler_wrap,
        ],
        interactors: vec![
            // component_interaction_handler_wrap,
        ],
        watchers: vec![],
    };
);

#[group]
#[summary = "Wolfram commands"]
#[commands(wolfram)]
struct Wolfram;

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

fn edit_handler_wrap(ctx: Context, msg_upd_event: MessageUpdateEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(edit_handler(ctx, msg_upd_event))
}

pub async fn edit_handler(ctx: Context, msg_upd_event: MessageUpdateEvent) {
    let inp_message = match msg_upd_event.channel_id.message(&ctx, msg_upd_event.id).await {
        Ok(m) => m,
        Err(_) => {return},
    };

    lazy_static! {
        static ref WOLFRAM_RE: Regex = Regex::new(format!(r"^{}wolfram (?P<args>.*)$", PREFIX).as_str()).unwrap();
        static ref ALIAS_RE: Regex = Regex::new(format!(r"^{}w (?P<args>.*)$", PREFIX).as_str()).unwrap();
    };

    let opts = vec![
        Opt::Format("image".to_string()),
        Opt::Output("json".to_string()),
    ];

    let arg: &str;

    if let Some(c) = WOLFRAM_RE.captures(&inp_message.content) {
        arg = c.name("args").unwrap().as_str();
    } else if let Some(c) = ALIAS_RE.captures(&inp_message.content) {
        arg = c.name("args").unwrap().as_str();
    } else {
        return
    }

    let lm = loading_msg(&ctx, &inp_message.channel_id).await.unwrap();

    let new_w = QueryResult::new(Opt::Input(arg.to_string()), opts).await.unwrap();
    let mut new_wm = WolfMessage::new(new_w.clone(), inp_message.clone(), new_w.pods).await;

    lm.delete(&ctx).await.unwrap();
    new_wm.send_messages(&ctx).await;
    push_to_editables(&ctx, Box::new(new_wm)).await;
    // push_to_interactables(&ctx, Box::new(new_wm)).await;
}

// pub fn component_interaction_handler_wrap(ctx: Context, interaction: Interaction) -> Pin<Box<dyn Future<Output = ()> + Send>> {
//     Box::pin(component_interaction_handler(ctx, interaction.message_component().unwrap()))
// }

// pub async fn component_interaction_handler(ctx: Context, interaction: MessageComponentInteraction) {
//     let message = match interaction.message {
//         InteractionMessage::Regular(m) => m,
//         _ => {return}
//     };
    
//     let user = match interaction.member {
//         Some(u) => u.user,
//         None => interaction.user,
//     };

//     let c = interaction.data;

//     match c.component_type {
//         ComponentType::Button => {
//             let wms_lock = {
//                 let data_read = ctx.data.read().await;
//                 data_read.get::<WolframMessages>().expect("Oops!").clone()  //TODO: Error handling
//             };

//             {
//                 let mut wms = wms_lock.write().await;
//                 wms.make_contiguous();
                
//                 for i in wms.iter_mut() {
//                     if i.inp_message.author != user {
//                         continue;
//                     }
//                     if i.header_message.as_ref().unwrap().id == message.id {
//                         match Buttons::from(c.custom_id.as_str()) {
//                             Buttons::Delete => {i.delete(&ctx).await;}
//                             // Buttons::Pod(_, n) => {i.pod_messages[n].send_message(ctx, message.channel_id).await.unwrap();},
//                             _ => {}
//                         }
//                     } else {
//                         for j in i.pod_messages.iter_mut() {
//                             if j.message.is_some() && j.message.as_ref().unwrap().id == message.id {
//                                 match Buttons::from(c.custom_id.as_str()) {
//                                     Buttons::Next => {
//                                         if j.curr_spod == (j.pod.subpods.len()-1) {
//                                             j.change_spod(&ctx, 0).await.unwrap();
//                                         } else {
//                                             j.change_spod(&ctx, j.curr_spod+1).await.unwrap();
//                                         }
//                                     },
//                                     Buttons::Prev => {
//                                         if j.curr_spod == 0 {
//                                             j.change_spod(&ctx, j.pod.subpods.len() -1).await.unwrap();
//                                         } else {
//                                             j.change_spod(&ctx, j.curr_spod-1).await.unwrap();
//                                         }
//                                     },
//                                     Buttons::Delete => {
//                                         j.delete_message(&ctx).await.unwrap();
//                                     },
//                                     _ => {}
//                                 }
//                             }
//                         }
//                     }
//                 }
                
//             }
//         },
//         ComponentType::SelectMenu => {
//             let wms_lock = {
//                 let data_read = ctx.data.read().await;
//                 data_read.get::<WolframMessages>().expect("Oops!").clone()  //TODO: Error handling
//             };

//             {
//                 let mut wms = wms_lock.write().await;
//                 wms.make_contiguous();
                
//                 for i in wms.iter_mut() {
//                     if i.inp_message.author != user {
//                         continue;
//                     }
//                     if i.header_message.as_ref().unwrap().id == message.id {
//                         for v in c.values.iter() {
//                             let pod_re = Regex::new(r"^POD(?P<n>\d+)").unwrap();
//                             if let Some(cap) = pod_re.captures(v) {
//                                 if let Some(n) = cap.name("n") {
//                                     let n = n.as_str().parse::<usize>().unwrap();
//                                     i.pod_messages[n].send_message(&ctx, message.channel_id).await.unwrap();
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         },
//         _ => {}
//     }

// }

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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

        let mut error = false;
        
        if let Some(b) = result["queryresult"]["error"].as_bool() {
            error = b;
        }
        
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

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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
                if m_items.len() == 0 {
                    if self.result.json["didyoumeans"].is_object() {
                        e.field("No result found!", format!("Did you mean:\n{}", self.result.json["didyoumeans"]["val"].as_str().unwrap()), false);
                    } else {
                        e.field("Uh oh", "No result found!", false);
                    }
                }
                e.footer(|f| {
                    if let Some(u) = self.inp_message.author.avatar_url() {
                        f.icon_url(u);
                    } else {
                        f.icon_url(self.inp_message.author.default_avatar_url());
                    }
                    f.text(format!("Requested by {}#{}", self.inp_message.author.name, self.inp_message.author.discriminator));
                    f
                });
                e
            });
            m.components(|c| {
                if m_items.len() > 0 {
                    MenuItem::add_menu(c, m_items, "POD");
                }
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

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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
        }).await?);

        Ok(())
    }
    
    async fn delete_message(&mut self, ctx: &Context) -> Result<(), errors::Error> {
        if let Some(m) = self.message.as_ref() {
            m.delete(ctx).await?;
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
            }).await?;
            self.curr_spod = spod;
        }
        Ok(())
    }
}

#[async_trait]
impl Editable for WolfMessage {
    async fn edit(&mut self, ctx: &Context) -> Result<(), errors::Error> {
        let old_m = self.header_message.clone();
        self.delete(&ctx).await;

        lazy_static! {
            static ref WOLFRAM_RE: Regex = Regex::new(format!(r"^{}wolfram (?P<args>.*)$", PREFIX).as_str()).unwrap();
            static ref ALIAS_RE: Regex = Regex::new(format!(r"^{}w (?P<args>.*)$", PREFIX).as_str()).unwrap();
        };

        let opts = vec![
            Opt::Format("image".to_string()),
            Opt::Output("json".to_string()),
        ];

        let inp_message = self.inp_message.channel_id.message(&ctx, self.inp_message.id).await?;

        let mut arg = "";

        if let Some(c) = WOLFRAM_RE.captures(&inp_message.content) {
            arg = c.name("args").unwrap().as_str();
        } else if let Some(c) = ALIAS_RE.captures(&inp_message.content) {
            arg = c.name("args").unwrap().as_str();
        }

        let new_w = QueryResult::new(Opt::Input(arg.to_string()), opts).await?;
        let mut new_wm = WolfMessage::new(new_w.clone(), inp_message.clone(), new_w.pods).await;

        self = &mut new_wm;
        self.send_messages(&ctx).await;

        let interactables_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<Interactables>().expect("Oops!").clone() //TODO: Error handling
        };

        {
            let mut interactables = interactables_lock.write().await;
            interactables.make_contiguous();

            let mut pos: Option<usize> = None;
            
            'outer: for (p, i) in interactables.iter().enumerate() {
                for j in i.get_response_message_id() {
                    if let Some(m) = &old_m {
                        if m.id == j {
                            pos = Some(p);
                            break 'outer;
                        }
                    }
                }
            }

            if let Some(p) = pos {
                interactables[p] = Box::new(self.clone());
            }
        }
        
        return Ok(())
    }

    fn get_input_message_id(&self) -> serenity::model::id::MessageId {
        self.inp_message.id.clone()
    }

    fn get_response_message_id(&self) -> Vec<MessageId> {
        let mut retvec: Vec<MessageId> = vec![];
        if let Some(m) = &self.header_message {
            retvec.push(m.id.clone());
        }
        for i in &self.pod_messages {
            if let Some(m) = &i.message {
                retvec.push(m.id.clone());
            }
        }
        return retvec;
    }

    fn get_command_pattern(&self) -> Regex {
        lazy_static! {
            static ref WOLFRAM_RE: Regex = Regex::new(format!(r"^{}wolfram .*$", PREFIX).as_str()).unwrap();
            static ref ALIAS_RE: Regex = Regex::new(format!(r"^{}w .*$", PREFIX).as_str()).unwrap();
        };

        if WOLFRAM_RE.is_match(&self.inp_message.content) {
            return WOLFRAM_RE.clone();
        } else if ALIAS_RE.is_match(&self.inp_message.content) {
            return ALIAS_RE.clone();
        } else {
            return WOLFRAM_RE.clone();
        }
    }
}

#[async_trait]
impl Interactable for WolfMessage {
    async fn interaction_respond(&mut self, ctx: &Context, interaction: Interaction) -> Result<(), errors::Error> {
        let old_m = self.header_message.clone();
        let component_interaction = match interaction {
            Interaction::MessageComponent(m) => m,
            _ => {return Ok(())}
        };
        
        component_interaction.create_interaction_response(ctx, |r|{
            r.kind(InteractionResponseType::UpdateMessage);
            r.interaction_response_data(|d|{
                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                d
            });
            r
        }).await?;

        let message = match &component_interaction.message {
            InteractionMessage::Regular(m) => m,
            InteractionMessage::Ephemeral(_) => {return Ok(())}
        };

        if self.inp_message.author != component_interaction.user {
            return Ok(())
        }

        match &component_interaction.data.component_type {
            ComponentType::Button => {
                for i in self.pod_messages.iter_mut() {
                    if i.message.is_some() && i.message.as_ref().unwrap().id == message.id {
                        match Buttons::from(component_interaction.data.custom_id.as_str()) {
                            Buttons::Delete => {
                                i.delete_message(&ctx).await?;
                            },
                            Buttons::Next => {
                                if i.curr_spod == (i.pod.subpods.len()-1) {
                                    i.change_spod(&ctx, 0).await?;
                                } else {
                                    i.change_spod(&ctx, i.curr_spod+1).await?;
                                }
                            },
                            Buttons::Prev => {
                                if i.curr_spod == 0 {
                                    i.change_spod(&ctx, i.pod.subpods.len()-1).await?;
                                } else {
                                    i.change_spod(&ctx, i.curr_spod-1).await?;
                                }
                            },
                            _ => {}
                        }
                    }
                }

                if self.header_message.is_some() && self.header_message.as_ref().unwrap().id == message.id {
                    match Buttons::from(component_interaction.data.custom_id.as_str()) {
                        Buttons::Delete => {
                            self.delete(&ctx).await;
                        },
                        _ => {}
                    }
                }
            },
            ComponentType::SelectMenu => {
                if self.header_message.is_some() && self.header_message.as_ref().unwrap().id == message.id {
                    for v in &component_interaction.data.values {
                        lazy_static! {
                            static ref POD_RE: Regex = Regex::new(r"^POD(?P<n>\d+)").unwrap();
                        }; 
                        if let Some(cap) = POD_RE.captures(&v) {
                            if let Some(n) = cap.name("n") {
                                let n = n.as_str().parse::<usize>().unwrap();
                                self.pod_messages[n].send_message(&ctx, message.channel_id).await?;
                            }
                        }
                    }
                }
            },
            _ => {}
        }

        let editables_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<Editables>().expect("Oops!").clone() //TODO: Error handling
        };

        {
            let mut editables = editables_lock.write().await;
            editables.make_contiguous();

            let mut pos: Option<usize> = None;
            
            'outer: for (p, i) in editables.iter().enumerate() {
                for j in i.get_response_message_id() {
                    if let Some(m) = &old_m {
                        if m.id == j {
                            pos = Some(p);
                            break 'outer;
                        }
                    }
                }
                if self.inp_message.id == i.get_input_message_id() {
                    pos = Some(p);
                }
            }

            if let Some(p) = pos {
                editables[p] = Box::new(self.clone());
            }
        }
        
        Ok(())
    }

    fn get_response_message_id(&self) -> Vec<MessageId> {
        let mut retvec: Vec<MessageId> = vec![];
        if let Some(m) = &self.header_message {
            retvec.push(m.id.clone());
        }
        for i in &self.pod_messages {
            if let Some(m) = &i.message {
                retvec.push(m.id.clone());
            }
        }
        return retvec;
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
            errors::err_msg(ctx, &msg.channel_id, Some(&lm), Some(&msg.author), &err).await?;
            Err(err)
        }
    }?;
    
    let opts = vec![
        Opt::Format("image".to_string()),
        Opt::Output("json".to_string()),
        ];
    
    let w = QueryResult::new(Opt::Input(query.to_string()), opts).await?;
    
    let mut wm = WolfMessage::new(w.clone(), msg.clone(), w.pods).await;

    lm.delete(&ctx.http).await?;
    
    if wm.result.json["error"].is_object() {
        if let Value::Object(a) = &wm.result.json["error"] {
            errors::err_msg(ctx, &msg.channel_id, Some(&lm), Some(&msg.author), &errors::Error::WolfError(a["msg"].to_string(), a["code"].to_string().parse::<u32>().unwrap())).await?;
        } 
    } else {
        wm.send_messages(ctx).await;
    }

    push_to_editables(&ctx, Box::new(wm.clone())).await;
    push_to_interactables(&ctx, Box::new(wm.clone())).await;
    
    Ok(())
}

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
        utils::loading_msg,
    },
    CONFIG,
    PREFIX
};
use serenity::{
    model::{
        channel::Message,
        id::ChannelId,
        prelude::MessageUpdateEvent,
    }, 
    prelude::{
        Context,
        TypeMapKey,
        RwLock,
    },
    framework::standard::
    {
        CommandResult, macros::command, Args,
    },
};
use serde_json::{
    Value
};
use urlencoding::encode;

pub const EDIT_BUFFER_SIZE: usize = 10;

#[derive(PartialEq)]
pub enum CmdType {
    Wolfram
}

lazy_static!{
    pub static ref REGMATCH: Vec<(Regex, CmdType)> = vec![
        (Regex::new(format!(r"^{}wolfram (?P<i>.*)$", PREFIX).as_str()).unwrap(), CmdType::Wolfram),
        (Regex::new(format!(r"^{}wolf (?P<i>.*)$", PREFIX).as_str()).unwrap(), CmdType::Wolfram),
    ];
}

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

#[derive(Clone)]
enum Opt {
    Podstate(String),
    Output(String),
    Input(String),
    Format(String),
}

impl Display for Opt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Opt::Podstate(s) => {write!(f, "podstate={}", encode(s))},
            Opt::Output(s) => {write!(f, "output={}", encode(s))},
            Opt::Input(s) => {write!(f, "input={}", encode(s))},
            Opt::Format(s) => {write!(f, "format={}", encode(s))},
        }
    }
}

#[derive(Clone)]
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

#[derive(Clone)]
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

#[derive(Clone)]
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

#[derive(Clone)]
pub struct Image {
    src: String,
    title: String,
    alt: String,
    img_type: String,
    invertable: bool,
    json: Value,
    img: Vec<u8>
}

impl Image {
    async fn new(json: &Value) -> Image {
        Image {
            src: json["src"].as_str().unwrap().to_string(),
            alt: json["alt"].as_str().unwrap().to_string(),
            title: json["title"].as_str().unwrap().to_string(),
            img_type: json["type"].as_str().unwrap().to_string(),
            invertable: json["colorinvertable"].as_bool().unwrap(),
            json: json.clone(),
            img: reqwest::get(json["src"].as_str().unwrap().to_string()).await.unwrap().bytes().await.unwrap().to_vec()
        }
    }
}

pub struct WolfMessage {
    result: QueryResult,
    inp_message: Message,
    header_message: Option<Message>,
    pod_messages: Vec<PodMessage>,
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
            m
        }).await.unwrap());
        
        for i in self.pod_messages.iter_mut() {
            i.send_message(ctx, self.inp_message.channel_id).await.unwrap();
        }
    } //TODO: Error handling
    
    async fn delete(&mut self, ctx: &Context) {
        self.header_message.as_ref().unwrap().delete(ctx).await.unwrap();

        for i in self.pod_messages.iter_mut() {
            i.message.as_ref().unwrap().delete(ctx).await.unwrap();
        }
    } //TODO: Error handling
}

pub struct PodMessage {
    pod: Pod,
    curr_spod: usize,
    message: Option<Message>
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
        self.message = Some(channel_id.send_message(&ctx.http, |m|{
            m.embed( |e| {
                e.title(&self.pod.title);
                e.image(&self.pod.subpods[0].image.src);
                e
            });
            m
        }).await.unwrap());

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
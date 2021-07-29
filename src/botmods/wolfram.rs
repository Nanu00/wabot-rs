use std::{
    fmt::Display,
    fmt,
};
use crate::{
    botmods::{
        errors,
        utils::loading_msg,
    },
    CONFIG,
};
use serenity::{
    model::{
        channel::Message,
        id::ChannelId,
    }, 
    prelude::Context,
    framework::standard::
    {
        CommandResult, macros::command, Args,
    },
};
use serde_json::{
    Value
};
use urlencoding::encode;

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

struct QueryResult {
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
        println!("{}", url);

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
struct Pod {
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
struct Subpod {
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
struct Image {
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

struct WolfMessage {
    result: QueryResult,
    inp_message: Message,
    header_message: Option<Message>,
    pod_messages: Vec<PodMessage>,
}

impl WolfMessage {
    async fn new(r: QueryResult, inp: Message) -> WolfMessage {
        WolfMessage{
            result: r,
            inp_message: inp,
            header_message: None,
            pod_messages: vec![]
        }
    }
}

struct PodMessage {
    pod: Pod,
    curr_spod: usize,
    message: Option<Message>
}

impl<'a> PodMessage {
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
    
    let mut wm = WolfMessage::new(w, msg.clone()).await;

    for i in wm.result.pods.iter() {
        wm.pod_messages.push(
            PodMessage::new(i).await
        );
    }

    lm.delete(&ctx.http).await?;
    
    wm.header_message = Some(msg.channel_id.send_message(&ctx.http, |m|{
        m.embed(|e| {
            e.title("Wolfram query");
            e.description("Results provided by [Wolfram|Alpha](https://www.wolframalpha.com/)");
            e.field("Input", &wm.result.input, false);
            e.footer(|f| {
                f.icon_url(msg.author.avatar_url().unwrap());
                f.text(format!("Requested by {}#{}", msg.author.name, msg.author.discriminator));
                f
            });
            e
        });
        m
    }).await?);
    
    for i in wm.pod_messages.iter_mut() {
        i.send_message(ctx, msg.channel_id).await?;
    }
    
    Ok(())
}
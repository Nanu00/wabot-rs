use std::{
    fmt::Display,
    fmt,
    collections::{
        HashSet,
    },
    borrow::Cow,
};
use crate::{
    botmods::{
        errors,
        utils::loading_msg,
    },
    CONFIG,
};
use serenity::{
    model::channel::Message, 
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command, Args,
    },
    http,
};
use serde_json::{
    Value
};

#[derive(Debug)]
struct Wolf {
    input: String,
    opts: Option<HashSet<Opt>>,
    result: Option<Value>,
    inp_message: Message,
    message: Option<Message>,
    error: Option<String>,
    pods: Vec<Pod>,
}

#[derive(Debug)]
struct Pod {
    title: String,
    scanner: String,
    id: String,
    error: bool,
    position: i64,
    numsubpods: i64,
    subpods: Vec<Subpod>,
}

#[derive(Debug)]
struct Subpod {
    title: String,
    image: Image,
    mathml: String,
}

#[derive(Debug)]
struct Image {
    src: String,
    alt: String,
    title: String,
    dimensions: (i64, i64),
    img_type: String,
    invertable: bool,
    img: Vec<u8>,
}

impl Image {
    async fn new(j: &serde_json::map::Map<String, Value>) -> Result<Image, errors::Error> {
        Ok(Image{
            src: j["src"].as_str().unwrap().to_string(),
            alt: j["alt"].as_str().unwrap().to_string(),
            title: j["title"].as_str().unwrap().to_string(),
            dimensions: (j["height"].as_i64().unwrap(), j["width"].as_i64().unwrap()),
            img_type: j["type"].as_str().unwrap().to_string(),
            invertable: j["colorinvertable"].as_bool().unwrap(),
            img: reqwest::get(j["src"].as_str().unwrap().to_string()).await?.bytes().await?.to_vec()
        })
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum Opt {
    Podstate(String),
    Output(String),
    Format(String),
}

impl Display for Opt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Opt::Podstate(s) => {write!(f, "podstate={}", s)},
            Opt::Output(s) => {write!(f, "output={}", s)},
            Opt::Format(s) => {write!(f, "format={}", s)},
        }
    }
}

impl Wolf {
    async fn new(input: String, inp_message: &Message) -> Wolf {
        Wolf {
            input,
            opts: None,
            result: None,
            inp_message: inp_message.clone(),
            message: None,
            error: None,
            pods: vec![],
        }
    }
    
    async fn get(&mut self, opts: Option<HashSet<Opt>>) -> Result<(), errors::Error> {
        let appid = {CONFIG.read().await.get::<String>("w_appid").unwrap()};
        let mut url = format!("https://api.wolframalpha.com/v2/query?appid={}&input={}", &appid, &self.input);
        if let Some(o) = opts {
            for i in o.iter() {
                url = format!("{}&{}", url, i);
                println!("{}", url);
            };
        };
        println!("{}", url);
        let result = reqwest::get(url).await?
            .json::<serde_json::Value>().await?;
        self.result = Some(result);
        Ok(())
    }
    
    async fn mkpods(&mut self) -> Result<(), errors::Error> {
        let pods = match &self.result {
            Some(r) => &r["queryresult"]["pods"],
            None => return Err(errors::Error::ArgError(0, 1)),
        };
        
        let mut ret_pods: Vec<Pod> = vec![];

        if let Some(ps) = pods.as_array() {
            for p in ps.iter() {
                let mut spods: Vec<Subpod> = Vec::new();
                for s in p["subpods"].as_array().unwrap().iter() {
                    println!("{:?}", s["mathml"]);
                    spods.push(
                        Subpod {
                            title: s["title"].as_str().unwrap().to_string(),
                            image: Image::new(&s["img"].as_object().unwrap()).await?,
                            mathml: s["mathml"].to_string(),
                        }
                    )
                }
                ret_pods.push(
                    Pod {
                        title: p["title"].as_str().unwrap().to_string(),
                        scanner: p["scanner"].as_str().unwrap().to_string(),
                        id: p["id"].as_str().unwrap().to_string(),
                        error: p["error"].as_bool().unwrap(),
                        position: p["position"].as_i64().unwrap(),
                        numsubpods: p["numsubpods"].as_i64().unwrap(),
                        subpods: spods,
                    }
                );
            }
        }
        
        self.pods = ret_pods;

        Ok(())
    }
}

#[command]
pub async fn wolfram(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {

    let lm = loading_msg(&ctx, &msg.channel_id).await?;

    let query = match arg.remains() {
        Some(r) => Ok(r),
        None => {
            let err = errors::Error::ArgError(1, 0);
            errors::err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &err).await?;
            Err(err)
        },
    }?;

    let mut w = Wolf::new(String::from(query), &msg).await;
    
    let mut opts: HashSet<Opt> = HashSet::new();
    opts.insert(Opt::Format("mathml".to_string()));
    opts.insert(Opt::Format("image".to_string()));
    opts.insert(Opt::Output("json".to_string()));

    w.get(Some(opts)).await?;
    w.mkpods().await.unwrap();
    
    let for_user = &msg.author;

    lm.delete(&ctx.http).await?;

    msg.channel_id.send_message(&ctx.http, |m|{
        m.embed(|e| {
            e.title("Wolfram query");
            e.description(format!("Input: {}", &w.input));
            e.image("attachment://image.png");
            e.footer(|f| {
                f.icon_url(for_user.avatar_url().unwrap());
                f.text(format!("Requested by {}#{}", for_user.name, for_user.discriminator));
                f
            });
            e
        });
        m.add_file(
            http::AttachmentType::Bytes {
                data: Cow::from(w.pods[0].subpods[0].image.img.clone()),
                filename: String::from("image.png")
            }
        );
        m
    }).await?;

    Ok(())
}
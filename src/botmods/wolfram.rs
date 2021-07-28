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
use regex::Regex;
use tokio::process::Command;
#[allow(unused_imports)] use usvg::SystemFontDB;
use usvg;
use tiny_skia::Color;

const SCALE: u32 = 8;

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
    mathml: Option<String>,
    mathml_img: Option<Vec<u8>>,
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

impl<'a> Pod {
    fn message_create<'b>(&self, m: &'b mut serenity::builder::CreateMessage<'a>) -> &'b mut serenity::builder::CreateMessage<'a> {
        // for (i, j) in self.subpods.iter().enumerate() {
            // m.add_file( http::AttachmentType::Bytes {
            //         data: Cow::from(j.image.img.clone()),
            //         filename: format!("{}-{}.gif", j.title, i)
            //     });
        // }
        m.add_file( http::AttachmentType::Bytes {
                data: Cow::from(self.subpods[0].image.img.clone()),
                filename: format!("{}-{}.gif", self.subpods[0].title, 1)
        });
        m.embed( |e| {
            e.title(&self.title);
            // for (i, j) in self.subpods.iter().enumerate() {
            //     e.image(format!("attachment://{}-{}.gif", j.title, i));
            // }
            e.image(format!("attachment://{}-{}.gif", self.subpods[0].title, 1));
            e
        });
        m
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

async fn mml_img(mml: &str) -> Result<Vec<u8>, errors::Error> {
    let regexes = [
        (Regex::new("\\\\n").unwrap(), "\n"),
        (Regex::new("\"").unwrap(), "\\\""),
        (Regex::new("integral").unwrap(), "&int;"),
        ];
    let mut mml = String::from(mml);
    
    for (i, j) in regexes.iter() {
        mml = i.replace_all(&mml, *j).into_owned();
    }
    
    let mj_cli = Command::new("sh")
        .arg("-c")
        .arg(format!("~/node_modules/.bin/mml2svg \"{}\"", &mml))
        .output()
        .await?;
    
    if !(mj_cli.status.success()) {
        let err = String::from_utf8(mj_cli.stderr).unwrap();
        return Err(errors::Error::ArgError(0, 0));   //TODO: New error types
    }
    
    let svg_raw = String::from_utf8(mj_cli.stdout).unwrap();
    let color_replace = Regex::new("currentColor").unwrap();
    let svg_raw = color_replace.replace_all(&svg_raw, "white");
    let svg_raw = svg_raw.as_bytes();
    
    let mut opt = usvg::Options::default();
    opt.fontdb.load_system_fonts();
    opt.fontdb.set_generic_families();
    let svg_tree = usvg::Tree::from_data(svg_raw, &opt)?;
    let pixmap_size = svg_tree.svg_node().size.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width()*SCALE, pixmap_size.height()*SCALE).unwrap();
    pixmap.fill(Color::BLACK);
    
    resvg::render(&svg_tree, usvg::FitTo::Zoom(SCALE as f32), pixmap.as_mut()).unwrap();

    Ok(pixmap.encode_png()?)
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
                    if s["mathml"].is_string() {
                        let mathml = s["mathml"].to_string();
                        // let mathml_img = match mml_img(&mathml[1..mathml.len()-1]).await {
                        //     Ok(i) => Some(i),
                        //     Err(_) => None,
                        // };

                        // Disabled this thing ^ cause I'm probably not going to use mathml
                        // still keeping the code here tho
                        
                        let mathml_img = None;
                        
                        spods.push( Subpod {
                                title: s["title"].as_str().unwrap().to_string(),
                                image: Image::new(&s["img"].as_object().unwrap()).await?,
                                mathml: Some(mathml),
                                mathml_img
                            })
                    } else {
                        spods.push( Subpod {
                                title: s["title"].as_str().unwrap().to_string(),
                                image: Image::new(&s["img"].as_object().unwrap()).await?,
                                mathml: None,
                                mathml_img: None
                            })
                    }
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
            e.description("Results provided by [Wolfram|Alpha](https://www.wolframalpha.com/)");
            e.footer(|f| {
                f.icon_url(for_user.avatar_url().unwrap());
                f.text(format!("Requested by {}#{}", for_user.name, for_user.discriminator));
                f
            });
            e
        });
        m
    }).await?;
    
    for i in w.pods.iter() {
        msg.channel_id.send_message(&ctx.http, |m| {
            i.message_create(m)
        }).await?;
    }

    Ok(())
}
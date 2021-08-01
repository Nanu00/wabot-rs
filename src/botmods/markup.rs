use serenity::{
    model::{
        channel::Message,
        event::MessageUpdateEvent,
        prelude::Interaction,
        interactions::{
            InteractionMessage,
            InteractionData,
            ComponentType
        }
    },
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command, Args,
    },
    http,
};
use std::{
    borrow::Cow,
    collections::VecDeque,
    sync::Arc,
};
#[allow(unused_imports)] use usvg::SystemFontDB;
use usvg;
use tiny_skia::Color;
use tempfile;
use crate::{
    botmods::{
        errors,
        errors::err_msg,
        utils::{
            loading_msg,
            Buttons,
            add_components,
        }
    },
    PREFIX
};
use regex::Regex;
use tokio::{
    process::Command,
};

const SCALE: u32 = 8;
pub const EDIT_BUFFER_SIZE: usize = 10;

#[derive(PartialEq)]
pub enum CmdType {
    Ascii,
    Latex,
    Inline
}

lazy_static!{
    pub static ref EDITMATCH: Vec<(Regex, CmdType)> = vec![
        (Regex::new(format!(r"^{}latex (?P<i>.*)$", PREFIX).as_str()).unwrap(), CmdType::Latex),
        (Regex::new(format!(r"^{}ascii (?P<i>.*)$", PREFIX).as_str()).unwrap(), CmdType::Ascii),
        (Regex::new(r"(\$.*\$)|(\\[.*\\])|(\\(.*\\))").unwrap(), CmdType::Inline),
    ];
    pub static ref COMPMATCH: Vec<Regex> = vec![
        Regex::new(format!(r"^{}latex .*$", PREFIX).as_str()).unwrap(),
        Regex::new(format!(r"^{}ascii .*$", PREFIX).as_str()).unwrap(),
        Regex::new(r"(\$.*\$)|(\\[.*\\])|(\\(.*\\))").unwrap(),
    ];
}

pub struct MathMessages;

impl TypeMapKey for MathMessages {
    type Value = Arc<RwLock<VecDeque<MathSnip>>>;
}

async fn math_messages_pusher(ctx: &Context, math_snip: MathSnip) {
    let math_messages_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<MathMessages>().expect("Oops!").clone() //TODO: Error handling
    };
    
    {
        let mut math_messages = math_messages_lock.write().await;
        math_messages.push_front(math_snip.clone());
        
        if math_messages.len() > EDIT_BUFFER_SIZE {
            math_messages.truncate(EDIT_BUFFER_SIZE);
        }
    }
}

pub async fn edit_handler(ctx: &Context, msg_upd_event: &MessageUpdateEvent, arg: &str, ct: &CmdType) {
    
    let inp_message = msg_upd_event.channel_id.message(&ctx, msg_upd_event.id).await.unwrap();

    let new_content = match &msg_upd_event.content {
        Some(c) => String::from(c),
        None => {return}
    };
    
    let new_text: Option<MathText>;
    
    if *ct == CmdType::Latex {
        new_text = Some(MathText::Latex(String::from(arg)));
    } else if *ct == CmdType::Ascii {
        new_text = Some(MathText::AsciiMath(String::from(arg)));
    } else if *ct == CmdType::Inline {
        new_text = Some(MathText::Latex(new_content));
    } else {
        return
    };
    
    let mut new_snip = MathSnip::new(new_text.unwrap(), &inp_message).await;
    let cmpl_result = new_snip.cmpl().await;

    let math_messages_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<MathMessages>().expect("Oops!").clone() //TODO: Error handling
    };
    
    {
        let mut math_messages = math_messages_lock.write().await;
        math_messages.make_contiguous();
        
        let mut msg_index: Option<usize> = None;
        
        for (i, j) in math_messages.iter().enumerate() {
            if j.inp_message.id == inp_message.id {
                msg_index = Some(i);
            };
        }
        
        let msg_index = match msg_index {
            Some(i) => i,
            None => {return}
        };
        
        let old_snip = math_messages.get(msg_index).unwrap();
        let old_msg = old_snip.message.clone().unwrap();
        
        old_msg.delete(&ctx).await.unwrap();
        // let new_msg = math_msg(&ctx, &inp_message.channel_id, None, &inp_message.author, &new_snip).await.unwrap();
        new_snip.message = match cmpl_result {
            Ok(_) => Some(math_msg(&ctx, &inp_message.channel_id, None, &inp_message.author, &new_snip).await.unwrap()),
            Err(e) => Some(err_msg(&ctx, &inp_message.channel_id, None, &inp_message.author, &e).await.unwrap()),
        };
        
        math_messages.insert(msg_index, new_snip);
        math_messages.remove(msg_index+1);
    }; //TODO: Error handling
}

pub async fn component_interaction_handler(ctx: &Context, interaction: Interaction) {
    let message = match interaction.message.unwrap() {
        InteractionMessage::Regular(m) => m,
        _ => {return}
    };
    
    let user = match interaction.member {
        Some(u) => u.user,
        None => interaction.user.unwrap(),
    };
    
    let math_messages_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<MathMessages>().expect("Oops!").clone() //TODO: Error handling
    };
    
    if let Some(InteractionData::MessageComponent(c)) = interaction.data {
        if let ComponentType::Button = c.component_type {
            {
                let mut math_messages = math_messages_lock.write().await;
                math_messages.make_contiguous();
                
                for j in math_messages.iter_mut() {
                    if j.message.is_some() && j.message.as_ref().unwrap().id == message.id && j.inp_message.author == user {
                        match Buttons::from(c.custom_id.as_str()) {
                            Buttons::Delete => {
                                j.message.as_ref().unwrap().delete(ctx).await.unwrap();
                            },
                            _ => {}
                        }
                    } else {
                        return
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum MathText {
    Latex(String),
    AsciiMath(String)
}

impl MathText {
    pub fn as_str(&self) -> &str {
        match self {
            MathText::Latex(s) => s,
            MathText::AsciiMath(s) => s,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MathSnip {
    text: MathText,
    image: Option<Vec<u8>>,
    inp_message: Message,
    pub message: Option<Message>,
    error:  Option<String>
}

impl MathSnip {
    pub async fn new(m_txt: MathText, i_msg: &Message) -> MathSnip {
        MathSnip {
            text: m_txt,
            image: None,
            inp_message: i_msg.clone(),
            message: None,
            error: None
        }
    }
    
    pub async fn cmpl(&mut self) -> Result<(), errors::Error> {
        let image = match &self.text {
            MathText::Latex(s) => {
                let tex_dir = tempfile::TempDir::new()?;

                let dvitex_cli = Command::new("sh")
                    .arg("-c")
                    .arg(format!("latex -interaction=nonstopmode -jobname=texput -output-directory={} '\\documentclass[preview,margin=1pt]{{standalone}} \\usepackage[utf8]{{inputenc}} \\usepackage{{mathtools}} \\usepackage{{siunitx}} \\usepackage[version=4]{{mhchem}} \\usepackage{{amsmath}} \\usepackage{{physics}} \\usepackage{{tikz-cd}} \\usepackage{{microtype}} \\usepackage{{xcolor}} \\begin{{document}} \\color{{white}} {} \\end{{document}}'", &tex_dir.path().to_str().unwrap(), &s))
                    .output()
                    .await?;
                
                if !(dvitex_cli.status.success()) {
                    let err = String::from_utf8(dvitex_cli.stdout).unwrap();
                    let useless = Regex::new(r"(?m)(^\(.+$\n)|(^This is .*$\n)|(^Document Class.*$\n)|(^No file.*$\n)|(^.* written on .*\.$\n)|(^\[1\].*$\n)|(^For additional .*$\n)|(^LaTeX2e .*$\n)|(^Preview.*$\n)|(^L3.*$\n)|(^ restricted \\write18 enabled\.$\n)|(^entering extended mode$\n)|(^dalone$\n)|(^.*\.dict\).*$\n)|(^*./usr/share.*$\n)|(^.*\.tex.*$\n)|(^[()]+$\n)").unwrap();
                    let err = useless.replace_all(&err, "").to_string();
                    
                    self.error = Some(err.clone());
                    return Err(errors::Error::MathError(err));
                }
                
                let dvisvg_cli = Command::new("sh")
                    .arg("-c")
                    .arg(format!("dvisvgm --page=1- -n --bbox=\"2pt\" -s {}", &tex_dir.path().join("texput.dvi").to_str().unwrap()))
                    .output()
                    .await?;
                
                if dvisvg_cli.status.success() {
                    dvisvg_cli.stdout
                } else {
                    return Err(errors::Error::MathError(String::from_utf8(dvisvg_cli.stderr).unwrap()))
                }
            },
            MathText::AsciiMath(s) => {
                let asm = String::from(s);
                
                let quote_escape = Regex::new("\"").unwrap();
                let asm = quote_escape.replace_all(&asm, "\\\"").to_string();
                
                let mjax_cli = Command::new("sh")
                    .arg("-c")
                    .arg(format!("~/node_modules/.bin/am2svg \"{}\"", &asm))
                    .output()
                    .await?;
                
                if !(mjax_cli.status.success()) {
                    let err = String::from_utf8(mjax_cli.stderr).unwrap();
                    self.error = Some(err.clone());
                    return Err(errors::Error::MathError(err));
                }
                
                let svg_raw = String::from_utf8(mjax_cli.stdout).unwrap();
                let color_replacer = Regex::new("currentColor").unwrap();
                let svg_raw = color_replacer.replace_all(&svg_raw, "white");
                
                svg_raw.as_bytes().to_vec()
            },
        };

        let mut opt = usvg::Options::default();
        opt.fontdb.load_system_fonts();
        opt.fontdb.set_generic_families();
        
        let svg_tree = usvg::Tree::from_data(&image, &opt)?;
        let pixmap_size = svg_tree.svg_node().size.to_screen_size();
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width()*SCALE, pixmap_size.height()*SCALE).unwrap();
        pixmap.fill(Color::BLACK);

        resvg::render(&svg_tree, usvg::FitTo::Zoom(SCALE as f32), pixmap.as_mut()).unwrap();
        
        self.image = Some(pixmap.encode_png()?);
        Ok(())
    }
}

async fn math_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId, loading_msg: Option<&Message>, for_user: &serenity::model::user::User, math: &MathSnip) -> Result<Message, SerenityError> {
    if let Some(m) = loading_msg {
        m.delete(&ctx.http).await?;
    }
    
    let buttons = vec![
        Buttons::Delete,
    ];

    c_id.send_message(&ctx.http, |m|{
        m.embed(|e| {
            e.title("Math snippet");
            e.description(format!("Input: {}", &math.text.as_str()));
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
                data: Cow::from(math.image.as_ref().unwrap()),
                filename: String::from("image.png")
            }
        );
        add_components(m, buttons);
        m
    }).await
}

#[command]
#[description = "Use this command to compile ASCIIMath to a PNG"]
pub async fn ascii(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {
    let lm = loading_msg(ctx, &msg.channel_id).await?;

    let asm_raw = match arg.remains() {
        Some(r) => Ok(r),
        None => {
            let err = errors::Error::ArgError(1, 0);
            err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &err).await?;
            Err(err)
        },
    }?;
    
    let mut asm = MathSnip::new(MathText::AsciiMath(String::from(asm_raw)), &msg).await;
    
    asm.message = match asm.cmpl().await {
        Ok(_) => Some(math_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &asm).await?),
        Err(e) => Some(err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &e).await?),
    };
    
    math_messages_pusher(ctx, asm).await;

    Ok(())
}

#[command]
#[description = "Use this command to compile LaTeX to a PNG"]
pub async fn latex(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {
    let lm = loading_msg(ctx, &msg.channel_id).await?;

    let latex_raw = match arg.remains() {
        Some(r) => Ok(r),
        None => {
            let err = errors::Error::ArgError(1, 0);
            err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &err).await?;
            Err(err)
        },
    }?;
    
    let mut latex = MathSnip::new(MathText::Latex(String::from(latex_raw)), &msg).await;
    
    latex.message = match latex.cmpl().await {
        Ok(_) => Some(math_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &latex).await?),
        Err(e) => Some(err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &e).await?),
    };

    math_messages_pusher(ctx, latex).await;

    Ok(())
}

pub async fn inline_latex(ctx: &Context, msg: &Message) -> CommandResult {
    let re_tex = fancy_regex::Regex::new(r"((\${1,2})(?![\s$]).+(?<![\s$])\2)|(\\[.*\\])|(\\(.*\\))").unwrap();
    let re_cmd = Regex::new(format!("{}{}{}", r"(^", PREFIX, r"latex.*)|(¯\\\\_(ツ)\\_/¯)").as_str()).unwrap();
    
    if re_tex.is_match(&msg.content).unwrap() && !re_cmd.is_match(&msg.content) {
        let lm = loading_msg(ctx, &msg.channel_id).await.unwrap();
        
        let mut latex = MathSnip::new(MathText::Latex(String::from(&msg.content)), &msg).await;
        // latex.cmpl().await

        latex.message = match latex.cmpl().await {
            Ok(_) => Some(math_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &latex).await?),
            Err(e) => Some(err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &e).await?),
        };

        math_messages_pusher(ctx, latex).await;
    };
    Ok(())
}
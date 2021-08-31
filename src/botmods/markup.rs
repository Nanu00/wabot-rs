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
    http,
    model::{
        channel::Message,
        event::MessageUpdateEvent,
        interactions::{
            Interaction,
            InteractionResponseType,
            InteractionApplicationCommandCallbackDataFlags,
        },
        id::MessageId,
    },
    prelude::*,
};
use std::{
    borrow::Cow,
    collections::VecDeque,
    sync::Arc,
    pin::Pin,
};
use futures::Future;
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
            BotModule,
            Editable,
            Interactable,
            push_to_editables,
            push_to_interactables
        },
    },
    PREFIX,
    Interactables,
    Editables
};
use regex::Regex;
use tokio::process::Command;
use serde::{
    Serialize,
    Deserialize
};
use lazy_static;

lazy_static!(
    pub static ref MOD_MARKUP: BotModule = BotModule {
        command_group: &MARKUP_GROUP,
        command_pattern: vec![
            Regex::new(format!(r"^{}latex .*$", PREFIX).as_str()).unwrap(),
            Regex::new(format!(r"^{}ascii .*$", PREFIX).as_str()).unwrap(),
            Regex::new(r"(\$.*\$)|(\\[.*\\])|(\\(.*\\))").unwrap()
        ],
        editors: vec![
           edit_handler_wrap,
        ],
        interactors: vec![
            // component_interaction_handler_wrap,
        ],
        watchers: vec![
            inline_latex_wrap,
        ],
    };
);

#[group]
#[summary = "Math formatting commands"]
#[commands(ascii, latex)]
struct Markup;

const SCALE: u32 = 8;

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

fn edit_handler_wrap(ctx: Context, msg_upd_event: MessageUpdateEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(edit_handler(ctx, msg_upd_event))
}

async fn edit_handler(ctx: Context, msg_upd_event: MessageUpdateEvent) {
    lazy_static! {
        static ref INLINE_RE: fancy_regex::Regex = fancy_regex::Regex::new(r"((\${1,2})(?![\s$]).+(?<![\s$])\2)|(\\[.*\\])|(\\(.*\\))").unwrap();
        static ref LATEX_RE: Regex = Regex::new(format!(r"^{}latex (?P<args>.*)$", PREFIX).as_str()).unwrap();
        static ref ASCII_RE: Regex = Regex::new(format!(r"^{}ascii (?P<args>.*)$", PREFIX).as_str()).unwrap();
    };

    let inp_message = match msg_upd_event.channel_id.message(&ctx, msg_upd_event.id).await {
        Ok(m) => m,
        Err(_) => {return},
    };

    let new_content = match &msg_upd_event.content {
        Some(c) => String::from(c),
        None => {return}
    };

    let mut ct: Option<CmdType> = None;
    let mut arg = "";

    if INLINE_RE.captures(&new_content).unwrap().is_some() && INLINE_RE.captures(&new_content).unwrap().unwrap().name("args").is_some() {
        arg = INLINE_RE.captures(&new_content).unwrap().unwrap().name("args").unwrap().as_str();
        ct = Some(CmdType::Inline);
    } else if LATEX_RE.captures(&new_content).is_some() && LATEX_RE.captures(&new_content).unwrap().name("args").is_some() {
        arg = LATEX_RE.captures(&new_content).unwrap().name("args").unwrap().as_str();
        ct = Some(CmdType::Latex);
    } else if ASCII_RE.captures(&new_content).is_some() && ASCII_RE.captures(&new_content).unwrap().name("args").is_some() {
        arg = ASCII_RE.captures(&new_content).unwrap().name("args").unwrap().as_str();
        ct = Some(CmdType::Ascii);
    }
    
    let new_text = match ct {
        Some(CmdType::Ascii) => MathText::AsciiMath(String::from(arg)),
        Some(CmdType::Latex) => MathText::Latex(String::from(arg)),
        Some(CmdType::Inline) => MathText::Latex(String::from(arg)),
        _ => {return}
    };

    let mut new_snip = MathSnip::new(new_text, &inp_message).await;
    let _cmpl_result = new_snip.cmpl().await;

    math_msg(&ctx, &msg_upd_event.channel_id, None, &msg_upd_event.author.unwrap(), &new_snip).await.unwrap();
    push_to_interactables(&ctx, Box::new(new_snip.clone())).await;
    push_to_editables(&ctx, Box::new(new_snip.clone())).await;
}

// pub fn component_interaction_handler_wrap(ctx: Context, interaction: Interaction) -> Pin<Box<dyn Future<Output = ()> + Send>> {
//     Box::pin(component_interaction_handler(ctx, interaction.message_component().unwrap()))
// }

// async fn component_interaction_handler(ctx: Context, interaction: MessageComponentInteraction) {
//     let message = match interaction.message {
//         InteractionMessage::Regular(m) => m,
//         _ => {return}
//     };
    
//     let user = match interaction.member {
//         Some(u) => u.user,
//         None => interaction.user,
//     };
    
//     let math_messages_lock = {
//         let data_read = ctx.data.read().await;
//         data_read.get::<Interactables>().expect("Oops!").clone() //TODO: Error handling
//     };

//     let c = interaction.data;
    
//     if let ComponentType::Button = c.component_type {
//         {
//             let mut math_messages = math_messages_lock.write().await;
//             math_messages.make_contiguous();
            
//             for j in math_messages.iter_mut() {
//                 if j.message.is_some() && j.message.as_ref().unwrap().id == message.id && j.inp_message.author == user {
//                     match Buttons::from(c.custom_id.as_str()) {
//                         Buttons::Delete => {
//                             j.message.as_ref().unwrap().delete(&ctx).await.unwrap();
//                         },
//                         _ => {}
//                     }
//                 } else {
//                     return
//                 }
//             }
//         }
//     }
// }

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
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

        if let Some(()) = resvg::render(&svg_tree, usvg::FitTo::Zoom(SCALE as f32), pixmap.as_mut()) {
            self.image = Some(pixmap.encode_png()?);
        } else {
            return Err(errors::Error::NoImgError());
        }
        
        Ok(())
    }
}

#[async_trait]
impl Editable for MathSnip {
    async fn edit(&mut self, ctx: &Context) -> Result<(), errors::Error> {
        if let Some(m) = &self.message {
            lazy_static! {
                static ref INLINE_RE: fancy_regex::Regex = fancy_regex::Regex::new(r"((\${1,2})(?![\s$]).+(?<![\s$])\2)|(\\[.*\\])|(\\(.*\\))").unwrap();
                static ref LATEX_RE: Regex = Regex::new(format!(r"^{}latex (?P<args>.*)$", PREFIX).as_str()).unwrap();
                static ref ASCII_RE: Regex = Regex::new(format!(r"^{}ascii (?P<args>.*)$", PREFIX).as_str()).unwrap();
            };

            let old_m = m.clone();

            m.delete(&ctx).await?;

            if let Ok(im) = self.inp_message.channel_id.message(&ctx, self.inp_message.id).await {
                if im.content == "" {
                    return Ok(())
                } else if INLINE_RE.captures(&im.content).unwrap().is_some() && INLINE_RE.captures(&im.content).unwrap().unwrap().name("args").is_some() {
                    self.text = MathText::Latex(String::from(INLINE_RE.captures(&im.content).unwrap().unwrap().name("args").unwrap().as_str()));
                } else if LATEX_RE.captures(&im.content).is_some() && LATEX_RE.captures(&im.content).unwrap().name("args").is_some() {
                    self.text = MathText::Latex(String::from(LATEX_RE.captures(&im.content).unwrap().name("args").unwrap().as_str()));
                } else if ASCII_RE.captures(&im.content).is_some() && ASCII_RE.captures(&im.content).unwrap().name("args").is_some() {
                    self.text = MathText::Latex(String::from(ASCII_RE.captures(&im.content).unwrap().name("args").unwrap().as_str()));
                } else {
                    return Ok(())
                }

                self.cmpl().await?;

                match math_msg(&ctx, &self.inp_message.channel_id, None, &self.inp_message.author, &self).await {
                    Ok(m) => {
                        self.message = Some(m);
                    },
                    Err(e) => {
                        Err(e)?    //TODO: Fix
                    }
                }
            }

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
                    if old_m.id == j {
                        pos = Some(p);
                        break 'outer;
                    }
                }
            }

            if let Some(p) = pos {
                interactables[p] = Box::new(self.clone());
            }
        }

        }
        return Ok(())
    }

    fn get_response_message_id(&self) -> Vec<MessageId> {
        match &self.message {
            Some(m) => vec![m.id.clone()],
            None => vec![]
        }
    }

    fn get_input_message_id(&self) -> serenity::model::id::MessageId {
        self.inp_message.id.clone()
    }

    fn get_command_pattern(&self) -> Regex {
            lazy_static! {
                static ref INLINE_RE: fancy_regex::Regex = fancy_regex::Regex::new(r"((\${1,2})(?![\s$]).+(?<![\s$])\2)|(\\[.*\\])|(\\(.*\\))").unwrap();
                static ref LATEX_RE: Regex = Regex::new(format!(r"^{}latex (?P<args>.*)$", PREFIX).as_str()).unwrap();
                static ref ASCII_RE: Regex = Regex::new(format!(r"^{}ascii (?P<args>.*)$", PREFIX).as_str()).unwrap();
            };

            if INLINE_RE.captures(&self.inp_message.content).unwrap().is_some() && INLINE_RE.captures(&self.inp_message.content).unwrap().unwrap().name("args").is_some() {
                return MOD_MARKUP.command_pattern[2].clone()
            } else if LATEX_RE.captures(&self.inp_message.content).is_some() && LATEX_RE.captures(&self.inp_message.content).unwrap().name("args").is_some() {
                return MOD_MARKUP.command_pattern[0].clone()
            } else if ASCII_RE.captures(&self.inp_message.content).is_some() && ASCII_RE.captures(&self.inp_message.content).unwrap().name("args").is_some() {
                return MOD_MARKUP.command_pattern[1].clone()
            } else {
                return MOD_MARKUP.command_pattern[2].clone()
            }
    }
}

#[async_trait]
impl Interactable for MathSnip {
    async fn interaction_respond(&mut self, ctx: &Context, interaction: Interaction) -> Result<(), errors::Error> {
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

        let old_m = self.message.clone();

        if let Buttons::Delete = Buttons::from(component_interaction.data.custom_id.as_str()) {
            if self.inp_message.author == component_interaction.user {
                self.message.as_ref().unwrap().channel_id.delete_message(&ctx, self.message.as_ref().unwrap().id).await?;
                self.message = None;
            }
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
        if let Some(m) = &self.message {
            return vec![m.id.clone()]
        } else {
            return vec![]
        }
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
                if let Some(a) = for_user.avatar_url() {
                    f.icon_url(a);
                } else {
                    f.icon_url(for_user.default_avatar_url());
                }
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
        m.components(|c| {
            Buttons::add_buttons(c, buttons);
            c
        });
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
            err_msg(ctx, &msg.channel_id, Some(&lm), Some(&msg.author), &err).await?;
            Err(err)
        },
    }?;
    
    let mut asm = MathSnip::new(MathText::AsciiMath(String::from(asm_raw)), &msg).await;
    
    asm.message = match asm.cmpl().await {
        Ok(_) => Some(math_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &asm).await?),
        Err(e) => Some(err_msg(ctx, &msg.channel_id, Some(&lm), Some(&msg.author), &e).await?),
    };
    
    push_to_interactables(&ctx, Box::new(asm.clone())).await;
    push_to_editables(&ctx, Box::new(asm.clone())).await;

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
            err_msg(ctx, &msg.channel_id, Some(&lm), Some(&msg.author), &err).await?;
            Err(err)
        },
    }?;
    
    let mut latex = MathSnip::new(MathText::Latex(String::from(latex_raw)), &msg).await;
    
    latex.message = match latex.cmpl().await {
        Ok(_) => Some(math_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &latex).await?),
        Err(e) => Some(err_msg(ctx, &msg.channel_id, Some(&lm), Some(&msg.author), &e).await?),
    };

    push_to_interactables(&ctx, Box::new(latex.clone())).await;
    push_to_editables(&ctx, Box::new(latex.clone())).await;

    Ok(())
}

pub fn inline_latex_wrap(ctx: Context, msg: Message) -> Pin<Box<dyn Future<Output = CommandResult> + Send>> {
    Box::pin(inline_latex(ctx, msg))
}

async fn inline_latex(ctx: Context, msg: Message) -> CommandResult {
    let re_tex = fancy_regex::Regex::new(r"((\${1,2})(?![\s$]).+(?<![\s$])\2)|(\\[.*\\])|(\\(.*\\))").unwrap();
    let re_cmd = Regex::new(format!("{}{}{}", r"(^", PREFIX, r"latex.*)|(¯\\\\_(ツ)\\_/¯)").as_str()).unwrap();
    
    if re_tex.is_match(&msg.content).unwrap() && !re_cmd.is_match(&msg.content) {
        let lm = loading_msg(&ctx, &msg.channel_id).await?;
        
        let mut latex = MathSnip::new(MathText::Latex(String::from(&msg.content)), &msg).await;
        latex.cmpl().await?;

        math_msg(&ctx, &msg.channel_id, Some(&lm), &msg.author, &latex).await?;

        // latex.message = match latex.cmpl().await {
        //     Ok(_) => Some(math_msg(&ctx, &msg.channel_id, Some(&lm), &msg.author, &latex).await?),
        //     Err(e) => Some(err_msg(&ctx, &msg.channel_id, Some(&lm), Some(&msg.author), &e).await?),
        // };

        push_to_interactables(&ctx, Box::new(latex.clone())).await;
        push_to_editables(&ctx, Box::new(latex.clone())).await;
    };

    Ok(())
}

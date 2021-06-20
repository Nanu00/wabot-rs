use serenity::{
    model::{
        channel::Message,
        id::MessageId,
        id::ChannelId
    },
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command, Args,
    },
    http,
};
use std::borrow::Cow;
#[allow(unused_imports)] use usvg::SystemFontDB;
use usvg;
use tiny_skia::Color;
use tempfile;
use crate::botmods::errors;
use crate::botmods::errors::err_msg;
use regex::Regex;
use tokio::process::Command;


const SCALE: u32 = 8;

enum MathText {
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

struct MathSnip {
    text: MathText,
    image: Option<Vec<u8>>,
    inp_message: MessageId,
    message: Option<MessageId>,
    channel: ChannelId,
    error:  Option<String>
}

impl MathSnip {
    pub async fn new(m_txt: MathText, i_msg: &Message) -> MathSnip {
        MathSnip {
            text: m_txt,
            image: None,
            inp_message: i_msg.id,
            message: None,
            channel: i_msg.channel_id,
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
                    let useless = Regex::new(r"(?m)(^\(.+$\n)|(^This is .*$\n)|(^Document Class.*$\n)|(^No file.*$\n)|(^.* written on .*\.$\n)|(^\[1\].*$\n)|(^For additional .*$\n)|(^LaTeX2e .*$\n)|(^Preview.*$\n)|(^L3.*$\n)|(^ restricted \\write18 enabled\.$\n)|(^entering extended mode$\n)|(^dalone$\n)|(^\.dict\)$\n)").unwrap();
                    let err = useless.replace_all(&err, "").to_string();
                    
                    self.error = Some(err.clone());
                    return Err(errors::Error::LatexError(err));
                }
                
                let dvisvg_cli = Command::new("sh")
                    .arg("-c")
                    .arg(format!("dvisvgm --page=1- -n --bbox=\"2pt\" -s {}", &tex_dir.path().join("texput.dvi").to_str().unwrap()))
                    .output()
                    .await?;
                
                if dvisvg_cli.status.success() {
                    dvisvg_cli.stdout
                } else {
                    return Err(errors::Error::LatexError(String::from_utf8(dvisvg_cli.stderr).unwrap()))
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
                    return Err(errors::Error::AsciiMError(err));
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

async fn loading_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId) -> Result<Message, SerenityError> {
    c_id.send_message(&ctx.http, |m| {
        m.content("Doing stuff <a:loading:840650882286223371>");
        m
    }).await
}

async fn math_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId, loading_msg: &Message, for_user: &serenity::model::user::User, math: &MathSnip) -> Result<Message, SerenityError> {
    loading_msg.delete(&ctx.http).await?;

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
    asm.cmpl().await?;
    
    asm.message = match asm.image {
        Some(_) => Some(math_msg(ctx, &msg.channel_id, &lm, &msg.author, &asm).await?.id),
        None => Some(err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &errors::Error::AsciiMError(String::from(&asm.error.unwrap()))).await?.id),
    };

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
    latex.cmpl().await?;
    
    latex.message = match latex.image {
        Some(_) => Some(math_msg(ctx, &msg.channel_id, &lm, &msg.author, &latex).await?.id),
        None => Some(err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &errors::Error::AsciiMError(String::from(&latex.error.unwrap()))).await?.id),
    };

    Ok(())
}

pub async fn inline_latex(ctx: &Context, msg: &Message) -> CommandResult {
    let re_tex = Regex::new(r"(\$.*\$)|(\\[.*\\])|(\\(.*\\))").unwrap();
    let re_cmd = Regex::new(r"(^\^latex.*)|(¯\\\\_(ツ)\\_/¯)").unwrap();
    
    if re_tex.is_match(&msg.content) && !re_cmd.is_match(&msg.content) {
        let lm = loading_msg(ctx, &msg.channel_id).await.unwrap();
        
        let mut latex = MathSnip::new(MathText::Latex(String::from(&msg.content)), &msg).await;
        latex.cmpl().await?;

        let latex_msg = match latex.image {
            Some(_) => math_msg(ctx, &msg.channel_id, &lm, &msg.author, &latex).await?.id,
            None => err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &errors::Error::AsciiMError(String::from(&latex.error.unwrap()))).await?.id,
        };
        latex.message = Some(latex_msg);
    };
    Ok(())
}
use serenity::{
    model::{
        channel::Message,
        interactions::InteractionData,
    }, 
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command, Args,
    },
    collector::{
        ComponentInteractionCollectorBuilder,
    },
    http,
    futures::stream::StreamExt,
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
use std::time::Duration;


trait MathSnip {
    fn get_img_bytes(&self) -> &Vec<u8>;
    fn get_plaintext(&self) -> &str;
}


fn svgpng(svg_data: &[u8]) -> Result<Vec<u8>, errors::Error> {
        let mut opt = usvg::Options::default();
        opt.fontdb.load_system_fonts();
        opt.fontdb.set_generic_families();

        let svg_tree = usvg::Tree::from_data(svg_data, &opt)?;
        let pixmap_size = svg_tree.svg_node().size.to_screen_size();
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width()*8, pixmap_size.height()*8).unwrap();
        pixmap.fill(Color::BLACK);

        // println!("Ready to render");

        resvg::render(&svg_tree, usvg::FitTo::Zoom(8.0), pixmap.as_mut()).unwrap();

        Ok(
            pixmap.encode_png()?
        )
}


async fn loading_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId) -> Result<Message, SerenityError> {
    c_id.send_message(&ctx.http, |m| {
        m.content("Doing stuff <a:loading:840650882286223371>");
        m
    }).await
}

async fn math_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId, loading_msg: &Message, for_user: &serenity::model::user::User, math: impl MathSnip) -> Result<Message, SerenityError> {
    loading_msg.delete(&ctx.http).await?;

    let msg = c_id.send_message(&ctx.http, |m|{
        m.embed(|e| {
            e.title("Math snippet");
            e.description(format!("Input: {}", &math.get_plaintext()));
            e.image("attachment://testfile.png");
            e.footer(|f| {
                f.icon_url(for_user.avatar_url().unwrap());
                f.text(format!("Requested by {}#{}", for_user.name, for_user.discriminator));
                f
            });
            e
        });
        m.add_file(
            http::AttachmentType::Bytes {
                data: Cow::from(math.get_img_bytes()),
                filename: String::from("testfile.png")
            });
        m.components( |c| {
            c.create_action_row( |r| {
                r.create_button( |b| {
                    b.label("Delete");
                    b.style(serenity::model::interactions::ButtonStyle::Primary);
                    b.custom_id("del");
                    b.disabled(false);
                    b
                })
            })
        });
        m
    }).await?;
    
    let clctr = ComponentInteractionCollectorBuilder::new(&ctx)
        .message_id(msg.id)
        .channel_id(msg.channel_id)
        .timeout(Duration::from_secs(300))
        .await;
    
    // serenity::collector::CollectComponentInteraction::new(&ctx).filter()

    // clctr.poll_next();
    
    let msg_r = &msg;

    let _clcted: Vec<_> = clctr.then( |i| async move {
        let b = match i.data.as_ref().unwrap() {
            InteractionData::MessageComponent(m) => &m.custom_id,
            _ => "wee"
        };
        
        if b == "del" {
            let _ = msg_r.delete(&ctx).await;
        };
    }).collect().await; 

    Ok(msg)
}


// AsciiMath

pub struct AsciiMath {
    text: String,
    png_bytes: Vec<u8>
}

impl MathSnip for AsciiMath {
    fn get_img_bytes(&self) -> &Vec<u8> {
        &self.png_bytes
    }
    fn get_plaintext(&self) -> &str {
        &self.text
    }
}

impl AsciiMath {
    pub async fn asmpng(asm: &str) -> Result<AsciiMath, errors::Error> {
        let asm = String::from(asm);
        
        let apos_escape = Regex::new("\"").unwrap();
        
        let asm = apos_escape.replace_all(&asm, "\\\"").to_string();

        // println!("Made string: {}", asm);

        let mj_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("~/node_modules/.bin/am2svg \"{}\"", &asm))
            .output();

        let mj_cli = mj_cli.await?;
        
        if !(mj_cli.status.success()) {
            let err = String::from_utf8(mj_cli.stderr).unwrap();
            println!("{}", &err);
            return Err(errors::Error::AsciiMError(err));
        }

        // println!("Ran MathJax: {}", &mj_cli.stdout.len());

        let svg_raw = String::from_utf8(mj_cli.stdout).unwrap();
        let color_re = Regex::new("currentColor").unwrap();
        let svg_raw = color_re.replace_all(&svg_raw, "white");

        let png_raw = match svgpng(svg_raw.as_bytes()) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };
        
        // println!("Done");

        Ok(
            AsciiMath {
                text: asm,
                png_bytes: png_raw,
            }
        )
    }
}

#[command]
#[description = "Use this command to compile ASCIIMath to a PNG"]
pub async fn ascii(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    let lm = loading_msg(ctx, &msg.channel_id).await?;

    let asm_raw = match args.remains() {
        Some(r) => Ok(r),
        None => {
            let er = errors::Error::ArgError(1, 0);
            err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &er).await?;
            Err(er)
        },
    }?;

    let _asm = match AsciiMath::asmpng(asm_raw).await {
        Ok(a) => math_msg(ctx, &msg.channel_id, &lm, &msg.author, a).await?,
        Err(e) => err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &e).await?,
    };

    Ok(())
}



// Latex

struct Latex {
    text: String,
    png_bytes: Vec<u8>,
}

impl MathSnip for Latex {
    fn get_img_bytes(&self) -> &Vec<u8> {
        &self.png_bytes
    }
    fn get_plaintext(&self) -> &str {
        &self.text
    }
}

impl Latex {
    pub async fn texpng(tex: &str) -> Result<Latex, errors::Error> {
        let tex_dir = tempfile::TempDir::new()?;

        // println!("tex string {}", tex);

        let dvitex_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("latex -interaction=nonstopmode -jobname=texput -output-directory={} '\\documentclass[preview,margin=1pt]{{standalone}} \\usepackage[utf8]{{inputenc}} \\usepackage{{mathtools}} \\usepackage{{siunitx}} \\usepackage[version=4]{{mhchem}} \\usepackage{{amsmath}} \\usepackage{{physics}} \\usepackage{{tikz-cd}} \\usepackage{{microtype}} \\usepackage{{xcolor}} \\begin{{document}} \\color{{white}} {} \\end{{document}}'", &tex_dir.path().to_str().unwrap(), &tex))
            .output();
        
        let dvitex_cli = dvitex_cli.await?;
        
        if !(dvitex_cli.status.success()) {
            let err = String::from_utf8(dvitex_cli.stdout).unwrap();
            let useless = Regex::new(r"(?m)(^\(.+$\n)|(^This is .*$\n)|(^Document Class.*$\n)|(^No file.*$\n)|(^.* written on .*\.$\n)|(^\[1\].*$\n)|(^For additional .*$\n)|(^LaTeX2e .*$\n)|(^Preview.*$\n)|(^L3.*$\n)|(^ restricted \\write18 enabled\.$\n)|(^entering extended mode$\n)|(^dalone$\n)|(^\.dict\)$\n)").unwrap();
            
            let err = useless.replace_all(&err, "").to_string();
            
            println!("{}", &err);
            
            return Err(errors::Error::LatexError(err));
        }

        // println!("dvi made {}\n{}", &tex_dir.path().join("texput.dvi").to_str().unwrap(), String::from_utf8(_dvitex_cli.stdout.clone()).unwrap());

        let dvisvg_cli = Command::new("sh")
        .arg("-c")
        .arg(format!("dvisvgm --page=1- -n --bbox=\"2pt\" -s {}", &tex_dir.path().join("texput.dvi").to_str().unwrap()))
        .output();
        
        let dvisvg_cli = dvisvg_cli.await?;

        // println!("svg made");
        if dvisvg_cli.status.success() {
            let png_raw = match svgpng(&dvisvg_cli.stdout) {
                Ok(r) => r,
                Err(e) => return Err(e),
            };

            Ok(
                Latex {
                    text: String::from(tex),
                    png_bytes: png_raw,
                }
                )
        } else {
            Err(errors::Error::LatexError(String::from_utf8(dvisvg_cli.stderr).unwrap()))
        }
    }
}

#[command]
#[description = "Use this command to compile LaTeX to a PNG"]
pub async fn latex(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    let lm = loading_msg(ctx, &msg.channel_id).await?;

    let latex_raw = match args.remains() {
        Some(r) => Ok(r),
        None => {
            let er = errors::Error::ArgError(1, 0);
            err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &er).await?;
            Err(er)
        },
    }?;

    let _latex = match Latex::texpng(latex_raw).await {
        Ok(l) => math_msg(ctx, &msg.channel_id, &lm, &msg.author, l).await?,
        Err(e) => err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &e).await?,
    };

    Ok(())
}

pub async fn inline_latex(ctx: &Context, msg: &Message) {
    let re_tex = Regex::new(r"(\$.*\$)|(\\[.*\\])|(\\(.*\\))").unwrap();
    let re_cmd = Regex::new(r"(^\^latex.*)|(¯\\\\_(ツ)\\_/¯)").unwrap();

    if re_tex.is_match(&msg.content) && !re_cmd.is_match(&msg.content) {
        let lm = loading_msg(ctx, &msg.channel_id).await.unwrap();
        let _latex = match Latex::texpng(&msg.content).await {
            Ok(l) => math_msg(ctx, &msg.channel_id, &lm, &msg.author, l).await,
            Err(e) => err_msg(ctx, &msg.channel_id, Some(&lm), &msg.author, &e).await,
        };
    }
}
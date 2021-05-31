use serenity::{
    model::channel::Message, 
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command, Args,
    },
    http,
};
use std::{
    process::Command,
    borrow::Cow,
};
#[allow(unused_imports)] use usvg::SystemFontDB;
use usvg;
use tiny_skia::Color;
use tempfile;
use crate::botmods::errors;


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
        pixmap.fill(Color::TRANSPARENT);

        // println!("Ready to render");

        resvg::render(&svg_tree, usvg::FitTo::Zoom(8.0), pixmap.as_mut()).unwrap();

        Ok(
            pixmap.encode_png()?
        )
}


async fn loading_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId) -> Result<Message, SerenityError> {
    c_id.send_message(&ctx.http, |m| {
        m.content("Doing stuff");
        m
    }).await
}


async fn math_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId, loading_msg: &Message, for_user: &serenity::model::user::User, math: impl MathSnip) -> Result<Message, SerenityError> {
    loading_msg.delete(&ctx.http).await?;

    c_id.send_message(&ctx.http, |m|{
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
        m
    }).await
}

pub async fn err_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId, loading_msg: &Message, for_user: &serenity::model::user::User, er: &errors::Error) -> Result<Message, SerenityError> {
    loading_msg.delete(&ctx.http).await?;
    
    c_id.send_message(&ctx.http, |m|{
        m.embed(|e| {
            e.title("Error");
            e.description(format!("There was an error: {}", er));
            e.footer(|f| {
                f.icon_url(for_user.avatar_url().unwrap());
                f.text(format!("Requested by {}#{}", for_user.name, for_user.discriminator));
                f
            });
            e
        });
        m
    }).await
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
    pub fn asmpng(asm: &str) -> Result<AsciiMath, errors::Error> {
        let mut asm = String::from(asm);
        asm.insert(0, '\'');
        asm.push('\'');

        // println!("Made string: {}", asm);

        let mj_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("~/node_modules/.bin/am2svg {} | sed -e 's/currentColor/white/g'", &asm))
            .output()?;

        // println!("Ran MathJax: {}", mj_cli.stdout.len());

        let png_raw = match svgpng(&mj_cli.stdout) {
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
pub async fn ascii(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    let lm = loading_msg(ctx, &msg.channel_id).await?;

    let asm_raw = match args.remains() {
        Some(r) => Ok(r),
        None => {
            let er = errors::Error::ArgError(1, 0);
            err_msg(ctx, &msg.channel_id, &lm, &msg.author, &er).await?;
            Err(er)
        },
    }?;

    let asm = AsciiMath::asmpng(asm_raw)?;

    math_msg(ctx, &msg.channel_id, &lm, &msg.author, asm).await?;

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
    pub fn texpng(tex: &str) -> Result<Latex, errors::Error> {
        let tex_dir = tempfile::TempDir::new()?;

        // println!("tex string {}", tex);

        let _dvitex_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("latex -jobname=texput -output-directory={} '\\documentclass[preview,margin=1pt]{{standalone}} \\usepackage[utf8]{{inputenc}} \\usepackage{{mathtools}} \\usepackage{{siunitx}} \\usepackage[version=4]{{mhchem}} \\usepackage{{amsmath}} \\usepackage{{xcolor}} \\begin{{document}} \\color{{white}} \\begin{{equation*}} {} \\end{{equation*}} \\end{{document}}'", &tex_dir.path().to_str().unwrap(), &tex))
            .output()?;

        // println!("dvi made {}\n{}", &tex_dir.path().join("texput.dvi").to_str().unwrap(), String::from_utf8(_dvitex_cli.stdout.clone()).unwrap());

        let dvisvg_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("dvisvgm --page=1- -n --bbox=\"2pt\" -s {}", &tex_dir.path().join("texput.dvi").to_str().unwrap()))
            .output()?;

        // println!("svg made");

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
    }
}

#[command]
pub async fn latex(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    let lm = loading_msg(ctx, &msg.channel_id).await?;

    let latex_raw = match args.remains() {
        Some(r) => Ok(r),
        None => {
            let er = errors::Error::ArgError(1, 0);
            err_msg(ctx, &msg.channel_id, &lm, &msg.author, &er).await?;
            Err(er)
        },
    }?;

    let latex = Latex::texpng(latex_raw)?;

    math_msg(ctx, &msg.channel_id, &lm, &msg.author, latex).await?;

    Ok(())
}
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
use tokio;
#[allow(unused_imports)] use usvg::SystemFontDB;
use usvg;
use tiny_skia::Color;

pub struct AsciiMath {
    text: String,
    byte_data: Vec<u8>
}
use tempfile;


// AsciiMath

impl AsciiMath {
    pub fn asmpng(asm: &str) -> Result<AsciiMath, Box<dyn std::error::Error + Send + Sync>> {
        let mut asm = String::from(asm);
        asm.insert(0, '\'');
        asm.push('\'');

        println!("Made string: {}", asm);

        let mj_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("~/node_modules/.bin/am2svg {}", &asm))
            .output()?;

        println!("Ran MathJax: {}", mj_cli.stdout.len());

        let mut opt = usvg::Options::default();
        opt.fontdb.load_system_fonts();
        opt.fontdb.set_generic_families();

        let svg_tree = usvg::Tree::from_data(&mj_cli.stdout, &opt)?;
        let pixmap_size = svg_tree.svg_node().size.to_screen_size();
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width()*4, pixmap_size.height()*4).unwrap();
        pixmap.fill(Color::WHITE);

        println!("Ready to render");

        resvg::render(&svg_tree, usvg::FitTo::Zoom(4.0), pixmap.as_mut()).unwrap();

        let png_raw = pixmap.encode_png()?;
        
        println!("Done");

        Ok(
            AsciiMath {
                text: asm,
                byte_data: png_raw,
            }
        )
    }
}

#[command]
pub async fn ascii(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let asm_raw = match args.remains() {
        Some(r) => Ok(r),
        None => Err("No arguments provided"),
    }?;

    let asm = AsciiMath::asmpng(asm_raw)?;

    msg.channel_id.send_message(&ctx.http, |m| {
        // m.content("Test");
        m.add_file(
            http::AttachmentType::Bytes {
                data: Cow::from(asm.byte_data),
                filename: String::from("testfile.png")
            });
        m
    }).await?;

    Ok(())
}



// Latex

struct Latex {
    latex: String,
    png_data: Vec<u8>,
}

impl Latex {
    pub fn texpng(tex: &str) -> Result<Latex, Box<dyn std::error::Error + Send + Sync>> {
        let tex_dir = tempfile::TempDir::new()?;

        let pre = "\\documentclass{standalone}
    \\usepackage{amsmath}
    \\begin{document}
    \\[
        ";

        let post = "
            \\]
            \\end{document}";

        println!("tex string {}", tex);

        let _dvitex_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("latex -jobname=texput -output-directory={} '\\documentclass[preview,margin=1pt]{{standalone}} \\usepackage[utf8]{{inputenc}} \\usepackage{{mathtools}} \\usepackage{{siunitx}} \\usepackage[version=4]{{mhchem}} \\usepackage{{amsmath}} \\usepackage{{xcolor}} \\begin{{document}} \\color{{white}} \\begin{{equation*}} {} \\end{{equation*}} \\end{{document}}'", &tex_dir.path().to_str().unwrap(), &tex))
            .output()?;

        println!("dvi made {}\n{}", &tex_dir.path().join("texput.dvi").to_str().unwrap(), String::from_utf8(_dvitex_cli.stdout.clone()).unwrap());

        let dvisvg_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("dvisvgm --page=1- -n --bbox=\"5pt\" -s {}", &tex_dir.path().join("texput.dvi").to_str().unwrap()))
            .output()?;

        println!("svg made");

        let mut opt = usvg::Options::default();
        opt.fontdb.load_system_fonts();
        opt.fontdb.set_generic_families();

        let svg_tree = usvg::Tree::from_data(&dvisvg_cli.stdout, &opt)?;
        let pixmap_size = svg_tree.svg_node().size.to_screen_size();
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width()*4, pixmap_size.height()*4).unwrap();
        pixmap.fill(Color::BLACK);

        println!("ready to render");

        resvg::render(&svg_tree, usvg::FitTo::Zoom(4.0), pixmap.as_mut()).unwrap();

        println!("rendered");

        let png_raw = pixmap.encode_png()?;

        Ok(
            Latex {
                latex: String::from(tex),
                png_data: png_raw,
            }
            )
    }
}

#[command]
pub async fn latex(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let latex_raw = match args.remains() {
        Some(r) => Ok(r),
        None => Err("No arguments provided"),
    }?;

    let latex = Latex::texpng(latex_raw)?;

    msg.channel_id.send_message(&ctx.http, |m| {
        // m.content("Test");
        m.add_file(
            http::AttachmentType::Bytes {
                data: Cow::from(latex.png_data),
                filename: String::from("testfile.png")
            }
            // http::AttachmentType::File {file: &latex.png_file, filename: String::from("test.png")}
            );
        m
    }).await?;

    Ok(())
}

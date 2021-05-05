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

pub struct AsciiMath {
    text: String,
    byte_data: Vec<u8>
}

impl AsciiMath {
    pub fn asmpng(asm: &str) -> Result<AsciiMath, Box<dyn std::error::Error + Send + Sync>> {
        let mut asm = String::from(asm);
        asm.insert(0, '\'');
        asm.push('\'');

        println!("Made string: {}", asm);

        let mut opt = usvg::Options::default();
        opt.fontdb.load_system_fonts();
        opt.fontdb.set_generic_families();

        let mj_cli = Command::new("sh")
            .arg("-c")
            .arg(format!("~/node_modules/.bin/am2svg {}", &asm))
            .output()?;

        println!("Ran MathJax: {}", mj_cli.stdout.len());

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

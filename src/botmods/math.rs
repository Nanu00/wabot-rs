use serenity::{
    model::channel::Message, 
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command, Args,
    }
};
use std::{
    process::Command,
    fs,
    io::Write,
};
use tempfile::tempfile;
use usvg::SystemFontDB;
use usvg;

pub struct AsciiMath {
    raw: String,
    file: fs::File,
}

impl AsciiMath {
    pub fn asmpng(asm: &str) -> Result<AsciiMath, Box<dyn std::error::Error>> {
        let mut asm = String::from(asm);
        asm.insert(0, '"');
        asm.push('"');

        let tmp = tempfile()?;

        let mut opt = usvg::Options::default();
        opt.fontdb.load_system_fonts();
        opt.fontdb.set_generic_families();

        let mj_cli = Command::new("$(npm bin)/am2svg")
            .arg(&asm)
            .output()?;

        let mj = String::from_utf8(mj_cli.stdout)?;
        let svg_tree = usvg::Tree::from_str(&mj, &opt)?;
        let pixmap_size = svg_tree.svg_node().size.to_screen_size();
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

        resvg::render(&svg_tree, usvg::FitTo::Original, pixmap.as_mut()).unwrap();

        let png_raw = pixmap.encode_png()?;
        tmp.write_all(&png_raw)?;

        Ok(
            AsciiMath {
                raw: asm,
                file: tmp,
            }
        )
    }
}

#[command]
pub async fn ascii(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let asm_raw = match args.remains() {
        Some(r) => Ok(r),
        None => Err("No arguments provided"),
    }?;

    let asm = AsciiMath::asmpng(asm_raw);

    // let mut asm = String::from(asm_raw);
    // asm.insert(0, '"');
    // asm.push('"');

    // let mut tmp = tempfile()?;

    // let mut opt = usvg::Options::default();
    // opt.fontdb.load_system_fonts();
    // opt.fontdb.set_generic_families();

    // let mj_cli = Command::new("$(npm bin)/am2svg")
    //     .arg(&asm)
    //     .output()?;

    // let mj = String::from_utf8(mj_cli.stdout)?;
    // let svg_tree = usvg::Tree::from_str(&mj, &opt)?;
    // let pixmap_size = svg_tree.svg_node().size.to_screen_size();
    // let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
 
    // resvg::render(&svg_tree, usvg::FitTo::Original, pixmap.as_mut()).unwrap();

    // let png_raw = pixmap.encode_png()?;
    // tmp.write_all(&png_raw)?;

    msg.channel_id.send_message(&ctx.http, |m| {m.content("Test")}).await?;

    Ok(())
}

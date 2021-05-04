// use serenity::{
//     model::channel::Message, 
//     prelude::*,
//     framework::standard::
//     {
//         CommandResult, macros::command
//     }
// };

use std::{
    process::Command,
    fs,
};

use tempfile::tempfile;
use usvg::SystemFontDB;
use usvg;

pub struct AsciiMath {
    raw: String,
    file: fs::File,
}

impl AsciiMath {
    pub fn asmpng(asm: &mut str) -> Result<AsciiMath, Box<dyn std::error::Error>> {
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

        Ok(
            AsciiMath {
                raw: asm,
                file: tmp,
            }
        )
    }
}

// use serenity::{
//     model::channel::Message, 
//     prelude::*,
//     framework::standard::
//     {
//         CommandResult, macros::command
//     }
// };

use std::{
    // process::Command,
    fs,
};

use tempfile::tempfile;
use resvg;

pub struct AsciiMath {
    raw: String,
    file: fs::File,
}

impl AsciiMath {
    async fn asmpng(asm: &str) -> Result<AsciiMath, Box<dyn std::error::Error>> {
        let tmp = tempfile::tempfile()?;
        let mut opt = resvg::usvg::Options::default();

        Ok(
            AsciiMath {
                raw: String::from(asm),
                file: tmp,
            }
        )
    }
}

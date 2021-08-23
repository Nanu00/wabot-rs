use std::{
    error::Error as StdErr,
    fmt::Display,
    fmt::Debug,
    fmt,
    io,
};
use usvg;
use png;
use serenity::{
    model::channel::Message, 
    prelude::*,
};
use reqwest;

pub async fn err_msg(ctx: &Context, c_id: &serenity::model::id::ChannelId, loading_msg: Option<&Message>, for_user: &serenity::model::user::User, er: &(impl StdErr + Display)) -> Result<Message, SerenityError> {
    if let Some(l) = loading_msg {
        l.delete(&ctx.http).await?;
    }

    let mut err_str = format!("There was an error:\n{}", er).to_string();

    if err_str.len() > 2000 {
        err_str.truncate(1997);
        err_str.push_str("...");
    };

    c_id.send_message(&ctx.http, |m|{
        m.embed(|e| {
            e.title("Error");
            e.description(err_str);
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

#[derive(Debug)]
pub enum Error {
    SVGError(usvg::Error),
    PNGError(png::EncodingError),
    IOError(io::Error),
    ArgError(u8, u8),
    MathError(String),
    RequestError(reqwest::Error),
    WolfError(String, u32)
}

impl From<usvg::Error> for Error {
    fn from(e: usvg::Error) -> Error {
        Error::SVGError(e)
    }
}

impl From<png::EncodingError> for Error {
    fn from(e: png::EncodingError) -> Error {
        Error::PNGError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::IOError(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::RequestError(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SVGError(e) => f.write_str(&format!("Error making the SVG: {}", e)),
            Error::PNGError(e) => f.write_str(&format!("Error making the PNG: {}", e)),
            Error::IOError(e) => f.write_str(&format!("I/O error: {}", e)),
            Error::ArgError(rec, need) => f.write_str(&format!("Expected {} argument(s), recieved {}", need, rec)),
            Error::MathError(e) => f.write_str(&format!("Compilation error:\n```{}```", e)),
            Error::RequestError(e) => f.write_str(&format!("Request error:\n{}", e)),
            Error::WolfError(e, c) => f.write_str(&format!("Wolfram error {} :\n{}", c, e))
        }
    }
}

impl StdErr for Error {
    fn source(&self) -> Option<&(dyn StdErr + 'static)> {
        match self {
            Error::SVGError(inner) => Some(inner),
            Error::PNGError(inner) => Some(inner),
            Error::IOError(inner) => Some(inner),
            _ => None,
        }
    }
}

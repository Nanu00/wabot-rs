use std::{
    error::Error as StdErr,
    fmt::Display,
    fmt::Debug,
    fmt,
    io,
};
use usvg;
use png;

#[derive(Debug)]
pub enum Error {
    SVGError(usvg::Error),
    PNGError(png::EncodingError),
    IOError(io::Error),
    ArgError(u8, u8),
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

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SVGError(e) => f.write_str(&format!("Error making the SVG: {}", e)),
            Error::PNGError(e) => f.write_str(&format!("Error making the PNG: {}", e)),
            Error::IOError(e) => f.write_str(&format!("I/O error: {}", e)),
            Error::ArgError(rec, need) => f.write_str(&format!("Expected {} argument(s), recieved {}", need, rec)),
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
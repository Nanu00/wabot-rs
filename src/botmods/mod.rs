use self::utils::BotModule;

pub mod general;
pub mod errors;
pub mod markup;
pub mod wolfram;
pub mod utils;
pub mod logging;

lazy_static!(
    pub static ref MODS: Vec<&'static BotModule> = vec![
        &self::general::MOD_GENERAL,
        &self::markup::MOD_MARKUP,
        &self::wolfram::MOD_WOLFRAM,
    ];
);

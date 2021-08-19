use serde::{
    Serialize,
    Deserialize
};
use chrono::prelude::{
    Utc,
    DateTime,
};
use ron::{
    ser::{
        to_string_pretty,
        PrettyConfig
    },
    de::from_reader,
};
use crate::{
    CONFIG,
    botmods::{
        wolfram::WolfMessage,
        markup::MathSnip,
    }
};
use std::{
    path::Path,
    fs::{
        File,
        metadata,
        write,
    },
};

#[derive(Serialize, Deserialize)]
enum CommandObj {
    Math(MathSnip),
    Wolf(WolfMessage)
}

#[derive(Serialize, Deserialize)]
pub struct LogObject {
    command_obj: CommandObj,
    timestamp: DateTime<Utc>
}

impl From<WolfMessage> for LogObject {
    fn from(w: WolfMessage) -> Self {
        LogObject {
            command_obj: CommandObj::Wolf(w),
            timestamp: Utc::now()
        }
    }
}

impl From<MathSnip> for LogObject {
    fn from(m: MathSnip) -> Self {
        LogObject {
            command_obj: CommandObj::Math(m),
            timestamp: Utc::now()
        }
    }
}

async fn log_read() -> Option<Vec<LogObject>> {
    let log_path = CONFIG.read().await.get::<String>("log_path").unwrap();
    let log_path = Path::new(&log_path);
    if metadata(&log_path).is_ok() {
        let f = File::open(&log_path).expect("Error opening log file!");
        let log: Vec<LogObject> = match from_reader(f) {
            Ok(x) => x,
            Err(e) => {panic!("Error reading log:\n{}", e)}
        };
        Some(log)
    } else {
        None
    }
}

pub async fn log_write(o: impl Into<LogObject>) {
    let log_path = CONFIG.read().await.get::<String>("log_path").unwrap();
    let log_path = Path::new(&log_path);
    let lo = o.into();
    let ron_cfg = PrettyConfig::new();

    if let Some(mut l) = log_read().await {
        l.push(lo);
        write(&log_path, to_string_pretty(&l, ron_cfg).unwrap()).unwrap();
    } else {
        let l = vec![lo];
        write(&log_path, to_string_pretty(&l, ron_cfg).unwrap()).unwrap();
    }
}

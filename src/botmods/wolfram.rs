use reqwest::async::Client;
use crate::botmods::errors;
use serenity::{
    model::channel::Message, 
    prelude::*,
    framework::standard::
    {
        CommandResult, macros::command, Args,
    },
    http,
};
use serde_json::Value;

struct Pod
    subpods: Vec<Pod>,
    vals: Vec<Value>,
};

struct WolframResult {
    r_client: Client,
    queries: HashMap,
    recd_json: String,
    pods: Vec<Pod>,
};
#![allow(dead_code)]

use clear_config::*;
use std::time::Duration;

#[derive(ConfigParser, Debug)]
enum Environment {
    Dev,
    Staging,
    #[config(rename = "Prod")]
    Production,
}

#[derive(ConfigReader, Debug)]
struct AppConfig {
    #[config(secret)]
    api_key: String,
    debug: bool,
    env: Environment,
    server: ServerConfig,
    #[config(default = "30 sec")]
    timeout: Duration,
}

#[derive(ConfigParser, Debug)]
struct Port(u16);

#[derive(ConfigReader, Debug)]
#[config(key_prefix = "SERVER_")]
struct ServerConfig {
    host: Option<String>,
    #[config(default = "8080")]
    port: Port,
}

fn main() {
    let mut ctx = ConfigContext::default();
    let result = ctx.read::<AppConfig>();
    println!("{}", ctx.report_used());
    match result {
        Ok(cfg) => println!("{:?}", cfg),
        Err(e) => println!("{}", e),
    }
}

#![allow(dead_code)]

use clear_config::*;
use std::time::Duration;

#[derive(ConfigParser, Debug)]
enum Environment {
    Dev,
    Staging,
    Prod,
}

#[derive(Debug)]
struct AppConfig {
    debug: bool,
    env: Environment,
    server: ServerConfig,
    timeout: Duration,
}

#[derive(Debug)]
struct ServerConfig {
    host: Option<String>,
    port: u16,
}

impl ConfigReader for ServerConfig {
    fn read(ctx: &mut ConfigContext) -> Option<Self> {
        let host = ctx.get::<String>("SERVER_HOST");
        let port = ctx.get_or_use::<u16>("SERVER_PORT", 8080);
        Some(ServerConfig {
            host: host?,
            port: port?,
        })
    }
}

impl ConfigReader for AppConfig {
    fn read(ctx: &mut ConfigContext) -> Option<Self> {
        let server = ServerConfig::read(ctx);
        let debug = ctx.need::<bool>("DEBUG");
        let timeout = ctx.get_or_parse::<Duration>("TIMEOUT", "30 sec");
        let env = ctx.need::<Environment>("ENV");
        Some(AppConfig {
            server: server?,
            debug: debug?,
            timeout: timeout?,
            env: env?,
        })
    }
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

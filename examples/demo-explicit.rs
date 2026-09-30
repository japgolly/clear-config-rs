use clear_config::*;
use std::time::Duration;

#[derive(ConfigParser, Debug)]
enum Environment {
    Dev,
    Staging,
    Prod,
}

#[allow(dead_code)]
#[derive(Debug)]
struct AppConfig {
    server: ServerConfig,
    debug: bool,
    timeout: Duration,
    env: Environment,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ServerConfig {
    host: Option<String>,
    port: u16,
}

impl ConfigReader for ServerConfig {
    fn read(ctx: &mut ConfigContext) -> Option<Self> {
        let host = ctx.get::<String>(&String::from("SERVER_HOST"));
        let port = ctx.get_or_use::<u16>(&String::from("SERVER_PORT"), 8080);
        Some(ServerConfig {
            host: host?,
            port: port?,
        })
    }
}

impl ConfigReader for AppConfig {
    fn read(ctx: &mut ConfigContext) -> Option<Self> {
        let server = ServerConfig::read(ctx);
        let debug = ctx.need::<bool>(&String::from("DEBUG"));
        let timeout = ctx.get_or_parse::<Duration>(&String::from("TIMEOUT"), "30 sec");
        let env = ctx.need::<Environment>(&String::from("ENV"));
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

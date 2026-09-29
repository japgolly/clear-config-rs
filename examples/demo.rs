use clear_config::*;

#[allow(dead_code)]
#[derive(Debug)]
struct ServerConfig {
    host: String,
    port: u16,
}

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
    verbose: bool,
    env: Environment,
}

impl ConfigDef for ServerConfig {
    fn load(ctx: &mut ConfigContext) -> Option<Self> {
        let host = ctx.get_or_parse::<String>(&String::from("HOST"), "localhost");
        let port = ctx.get_or_parse::<u16>(&String::from("PORT"), "8080");
        Some(ServerConfig {
            host: host?,
            port: port?,
        })
    }
}

impl ConfigDef for AppConfig {
    fn load(ctx: &mut ConfigContext) -> Option<Self> {
        let server = ServerConfig::load(ctx);
        let debug = ctx.need::<bool>(&String::from("DEBUG"));
        let verbose = ctx.need::<bool>(&String::from("VERBOSE"));
        let env = ctx.need::<Environment>(&String::from("ENV"));
        Some(AppConfig {
            server: server?,
            debug: debug?,
            verbose: verbose?,
            env: env?,
        })
    }
}

fn main() {
    let mut ctx = ConfigContext::default();
    let result = ctx.load::<AppConfig>();
    println!("{:?}", result);
}

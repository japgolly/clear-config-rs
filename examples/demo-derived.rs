use clear_config::*;

#[allow(dead_code)]
#[derive(ConfigReader, Debug)]
#[config(key_prefix = "SERVER_")]
struct ServerConfig {
    host: Option<String>,
    #[config(default = "8080")]
    port: u16,
}

#[derive(ConfigParser, Debug)]
enum Environment {
    Dev,
    Staging,
    Prod,
}

#[allow(dead_code)]
#[derive(ConfigReader, Debug)]
struct AppConfig {
    server: ServerConfig,
    debug: bool,
    #[config(default = "false")]
    verbose: bool,
    env: Environment,
}

fn main() {
    let mut ctx = ConfigContext::default();
    let result = ctx.load::<AppConfig>();
    println!("{}", ctx.report_used());
    match result {
        Ok(cfg) => println!("{:?}", cfg),
        Err(e) => println!("{}", e),
    }
}

use clear_config::*;

// #[allow(dead_code)]
// #[derive(Debug)]
// struct ServerConfig {
//     host: String,
//     port: u16,
// }

// enum Environment {
//     Dev, Staging, Prod
// }

#[allow(dead_code)]
#[derive(Debug)]
struct AppConfig {
    // server: ServerConfig,
    debug: bool,
    verbose: bool,
}

impl ConfigDef for AppConfig {
    fn load(ctx: &mut ConfigContext) -> Result<Self, Vec<LoadError>> {
        let mut errors: Vec<LoadError> = Vec::new();
        let debug = match ctx.need::<bool>(&String::from("DEBUG")) {
            Ok(v) => Some(v),
            Err(e) => {
                errors.push(e);
                None
            }
        };
        let verbose = match ctx.need::<bool>(&String::from("VERBOSE")) {
            Ok(v) => Some(v),
            Err(e) => {
                errors.push(e);
                None
            }
        };
        if errors.is_empty() {
            let debug = debug.unwrap();
            let verbose = verbose.unwrap();
            Ok(AppConfig { debug, verbose })
        } else {
            Err(errors)
        }
    }
}

fn main() {
    let mut ctx = ConfigContext::default();

    let cfg = AppConfig::load(&mut ctx);
    println!("Config: {:?}", cfg);
}

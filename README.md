# Clear Config for Rust

This is a library that

1. helps you load config (from environment vars by default)
1. provides a report to show you what was read and from where

# Usage

First declare your config and derive a `ConfigReader`.

This example sets up the following:

```
╭─────────────┬─────────╮
│ Key         │ Default │
├─────────────┼─────────┤
│ DEBUG       │         │ // Mandatory
│ ENV         │         │ // Mandatory
│ SERVER_HOST │         │ // Optional
│ SERVER_PORT │ 8080    │
│ TIMEOUT     │ 30 sec  │
╰─────────────┴─────────╯
```


```rs
use clear_config::*;
use std::time::Duration;

#[derive(ConfigParser, Debug)]
enum Environment {
    Dev,
    Staging,
    Prod,
}

#[derive(ConfigReader, Debug)]
struct AppConfig {
    server: ServerConfig,
    debug: bool,
    #[config(default = "30 sec")]
    timeout: Duration,
    env: Environment,
}

#[derive(ConfigReader, Debug)]
#[config(key_prefix = "SERVER_")]
struct ServerConfig {
    host: Option<String>,
    #[config(default = "8080")]
    port: u16,
}
```

Next, read from the environment and print out a report:

```rs
fn main() {
    let mut ctx = ConfigContext::default();
    let result = ctx.read::<AppConfig>();
    println!("{}", ctx.report_used());
    match result {
        Ok(cfg) => println!("{:?}", cfg),
        Err(e) => println!("{}", e),
    }
}
```

An example of the output printed is:

```
╭─────────────────────────────────────╮
│            Config Report            │
├─────────────┬─────────────┬─────────┤
│ Key         │ Environment │ Default │
├─────────────┼─────────────┼─────────┤
│ DEBUG       │ true        │         │
│ ENV         │ dev         │         │
│ SERVER_HOST │             │         │
│ SERVER_PORT │ 3000        │ 8080    │
│ TIMEOUT     │ 1min        │ 30 sec  │
╰─────────────┴─────────────┴─────────╯
AppConfig { server: ServerConfig { host: None, port: 3000 }, debug: true, timeout: 60s, env: Dev }
```

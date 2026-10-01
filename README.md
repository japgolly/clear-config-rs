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
│ DEBUG       │         │ Mandatory field
│ ENV         │         │ Mandatory field
│ SERVER_HOST │ None    │
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
    debug: bool,
    env: Environment,
    server: ServerConfig,
    #[config(default = "30 sec")]
    timeout: Duration,
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
│ SERVER_HOST │             │ None    │
│ SERVER_PORT │ 3000        │ 8080    │
│ TIMEOUT     │ 1min        │ 30 sec  │
╰─────────────┴─────────────┴─────────╯
AppConfig { server: ServerConfig { host: None, port: 3000 }, debug: true, timeout: 60s, env: Dev }
```

On the other hand, if something goes wrong you can expect to see a set of errors:

```
╭─────────────────────────────────────╮
│            Config Report            │
├─────────────┬─────────────┬─────────┤
│ Key         │ Environment │ Default │
├─────────────┼─────────────┼─────────┤
│ DEBUG       │             │         │
│ ENV         │ local       │         │
│ SERVER_HOST │             │ None    │
│ SERVER_PORT │             │ 8080    │
│ TIMEOUT     │ what        │ 30 sec  │
╰─────────────┴─────────────┴─────────╯
Config errors:
  * DEBUG: not specified
  * ENV: "local" is not a valid Environment (expected one of: Dev, Staging, Prod)
  * TIMEOUT: "what" is not a valid duration
```

## Loading a .env file

Instead of

```rs
    let mut ctx = ConfigContext::default();
```

you can manually specify config sources:

```rs
    let mut ctx = ConfigContext::new(vec![
        ConfigSource::env(),
        ConfigSource::env_file(".env"),
    ]);
```

Example output:

```
╭─────────────────────────────────────────────────╮
│                  Config Report                  │
├─────────────┬─────────────┬───────────┬─────────┤
│ Key         │ Environment │ .env      │ Default │
├─────────────┼─────────────┼───────────┼─────────┤
│ DEBUG       │ false       │ 1         │         │
│ ENV         │             │ dev       │         │
│ SERVER_HOST │             │ localhost │ None    │
│ SERVER_PORT │             │ 3000      │ 8080    │
│ TIMEOUT     │             │ 1min      │ 30 sec  │
╰─────────────┴─────────────┴───────────┴─────────╯
```

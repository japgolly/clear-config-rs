use clear_config::{ConfigContext, ConfigSource};
use clear_config_derive::{ConfigParser, ConfigReader};
use std::collections::HashMap;

fn make_context(entries: Vec<(&str, &str)>) -> ConfigContext {
    let mut data = HashMap::new();
    for (k, v) in entries {
        data.insert(k.to_string(), v.to_string());
    }
    ConfigContext::new(vec![ConfigSource {
        name: "Test".to_string(),
        data,
    }])
}

// 1. Test default field name in uppercase and required (need)
#[derive(ConfigReader, Debug, PartialEq)]
struct BasicConfig {
    host: String,
    port: u16,
    debug: bool,
}

#[test]
fn test_default_key_and_need_success() {
    let mut ctx = make_context(vec![
        ("HOST", "127.0.0.1"),
        ("PORT", "3000"),
        ("DEBUG", "true"),
    ]);

    let cfg = ctx.load::<BasicConfig>().unwrap();
    assert_eq!(
        cfg,
        BasicConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            debug: true,
        }
    );
}

#[test]
fn test_default_key_missing_error() {
    let mut ctx = make_context(vec![("HOST", "127.0.0.1")]);

    let err = ctx.load::<BasicConfig>().unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("PORT: not specified"));
    assert!(err_str.contains("DEBUG: not specified"));
}

// 2. Test custom key annotation
#[derive(ConfigReader, Debug, PartialEq)]
struct CustomKeyConfig {
    #[config(key = "APP_HOST_NAME")]
    host: String,
    #[config(key = "APP_PORT_NUMBER")]
    port: u16,
}

#[test]
fn test_custom_key() {
    let mut ctx = make_context(vec![
        ("APP_HOST_NAME", "example.com"),
        ("APP_PORT_NUMBER", "443"),
    ]);

    let cfg = ctx.load::<CustomKeyConfig>().unwrap();
    assert_eq!(
        cfg,
        CustomKeyConfig {
            host: "example.com".to_string(),
            port: 443,
        }
    );
}

// 3. Test default values (get_or_parse)
#[derive(ConfigReader, Debug, PartialEq)]
struct DefaultValueConfig {
    #[config(default = "localhost")]
    host: String,
    #[config(default = "8080")]
    port: u16,
    #[config(default = "true")]
    enabled: bool,
}

#[test]
fn test_default_values_used_when_missing() {
    let mut ctx = make_context(vec![]);

    let cfg = ctx.load::<DefaultValueConfig>().unwrap();
    assert_eq!(
        cfg,
        DefaultValueConfig {
            host: "localhost".to_string(),
            port: 8080,
            enabled: true,
        }
    );
}

#[test]
fn test_default_values_overridden_when_present() {
    let mut ctx = make_context(vec![
        ("HOST", "api.example.com"),
        ("PORT", "9090"),
        ("ENABLED", "false"),
    ]);

    let cfg = ctx.load::<DefaultValueConfig>().unwrap();
    assert_eq!(
        cfg,
        DefaultValueConfig {
            host: "api.example.com".to_string(),
            port: 9090,
            enabled: false,
        }
    );
}

// 4. Test custom key combined with default value
#[derive(ConfigReader, Debug, PartialEq)]
struct CustomKeyAndDefaultConfig {
    #[config(key = "SERVER_HOST", default = "127.0.0.1")]
    host: String,
    #[config(key = "SERVER_PORT", default = "5000")]
    port: u16,
}

#[test]
fn test_custom_key_and_default() {
    let mut ctx = make_context(vec![("SERVER_PORT", "9999")]);

    let cfg = ctx.load::<CustomKeyAndDefaultConfig>().unwrap();
    assert_eq!(
        cfg,
        CustomKeyAndDefaultConfig {
            host: "127.0.0.1".to_string(),
            port: 9999,
        }
    );
}

// 5. Test Option fields (get)
#[derive(ConfigReader, Debug, PartialEq)]
struct OptionConfig {
    host: String,
    port: Option<u16>,
    #[config(key = "CUSTOM_TAG")]
    tag: Option<String>,
}

#[test]
fn test_option_fields_missing_are_none() {
    let mut ctx = make_context(vec![("HOST", "localhost")]);

    let cfg = ctx.load::<OptionConfig>().unwrap();
    assert_eq!(
        cfg,
        OptionConfig {
            host: "localhost".to_string(),
            port: None,
            tag: None,
        }
    );
}

#[test]
fn test_option_fields_present_are_some() {
    let mut ctx = make_context(vec![
        ("HOST", "localhost"),
        ("PORT", "8080"),
        ("CUSTOM_TAG", "v1.0"),
    ]);

    let cfg = ctx.load::<OptionConfig>().unwrap();
    assert_eq!(
        cfg,
        OptionConfig {
            host: "localhost".to_string(),
            port: Some(8080),
            tag: Some("v1.0".to_string()),
        }
    );
}

#[test]
fn test_option_fields_invalid_produces_error() {
    let mut ctx = make_context(vec![("HOST", "localhost"), ("PORT", "not_a_number")]);

    let err = ctx.load::<OptionConfig>().unwrap_err();
    assert!(
        err.to_string()
            .contains("PORT: \"not_a_number\" is not a valid u16")
    );
}

// 6. Test nested structs (calling load)
#[derive(ConfigReader, Debug, PartialEq)]
struct DatabaseConfig {
    #[config(key = "DB_HOST", default = "db.internal")]
    host: String,
    #[config(key = "DB_PORT", default = "5432")]
    port: u16,
}

#[derive(ConfigParser, Debug, PartialEq)]
enum Environment {
    Dev,
    Prod,
}

#[derive(ConfigReader, Debug, PartialEq)]
struct AppConfig {
    db: DatabaseConfig,
    env: Environment,
    debug: bool,
    #[config(default = "false")]
    verbose: bool,
    log_file: Option<String>,
}

#[test]
fn test_nested_struct_and_all_features_combined() {
    let mut ctx = make_context(vec![
        ("DB_PORT", "5433"),
        ("ENV", "prod"),
        ("DEBUG", "true"),
        ("LOG_FILE", "/var/log/app.log"),
    ]);

    let cfg = ctx.load::<AppConfig>().unwrap();
    assert_eq!(
        cfg,
        AppConfig {
            db: DatabaseConfig {
                host: "db.internal".to_string(),
                port: 5433,
            },
            env: Environment::Prod,
            debug: true,
            verbose: false,
            log_file: Some("/var/log/app.log".to_string()),
        }
    );
}

#[test]
fn test_nested_struct_error_accumulation() {
    let mut ctx = make_context(vec![("DB_PORT", "not_a_port"), ("ENV", "invalid_env")]);

    let err = ctx.load::<AppConfig>().unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("DB_PORT"));
    assert!(err_str.contains("ENV"));
    assert!(err_str.contains("DEBUG: not specified"));
}

// 7. Test key_prefix attribute on struct
#[derive(ConfigReader, Debug, PartialEq)]
#[config(key_prefix = "SERVER_")]
struct PrefixedServerConfig {
    host: Option<String>,
    #[config(default = "8080")]
    port: u16,
    #[config(key = "TIMEOUT_SECONDS")]
    timeout: u32,
}

#[test]
fn test_key_prefix_on_struct() {
    let mut ctx = make_context(vec![
        ("SERVER_HOST", "192.168.1.1"),
        ("SERVER_TIMEOUT_SECONDS", "30"),
    ]);

    let cfg = ctx.load::<PrefixedServerConfig>().unwrap();
    assert_eq!(
        cfg,
        PrefixedServerConfig {
            host: Some("192.168.1.1".to_string()),
            port: 8080,
            timeout: 30,
        }
    );
}

#[test]
fn test_key_prefix_missing_required_field_error() {
    let mut ctx = make_context(vec![("SERVER_HOST", "192.168.1.1")]);

    let err = ctx.load::<PrefixedServerConfig>().unwrap_err();
    assert!(
        err.to_string()
            .contains("SERVER_TIMEOUT_SECONDS: not specified")
    );
}

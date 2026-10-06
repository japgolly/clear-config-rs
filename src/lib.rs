//! Clear Config is a lightweight configuration loading and reporting library for Rust.
//!
//! # Overview
//!
//! - Declare configuration structs with `#[derive(ConfigReader)]`.
//! - Derive enum and newtype parsers with `#[derive(ConfigParser)]`.
//! - Read from environment variables, `.env` files, or in-memory sources.
//! - Generate clear ASCII report tables showing what keys were loaded from where,
//!   with sensitive keys (passwords, tokens) automatically obfuscated.
//!
//! # Example
//!
//! ```rust
//! use clear_config::*;
//!
//! #[derive(ConfigReader, Debug)]
//! struct AppConfig {
//!     #[config(default = "8080")]
//!     port: u16,
//! }
//!
//! let mut ctx = ConfigContext::default();
//! let result = ctx.read::<AppConfig>();
//! println!("{}", ctx.report_used());
//! match result {
//!     Ok(cfg) => println!("{:?}", cfg),
//!     Err(e) => println!("{}", e),
//! }
//! ```
extern crate self as clear_config;

mod parsing;
mod source;

pub use crate::parsing::*;
pub use crate::source::*;
pub use clear_config_derive::{ConfigParser, ConfigReader};

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Display};

/// A trait for types that can be read from a [`ConfigContext`].
///
/// This trait is usually derived using `#[derive(ConfigReader)]` on named structs.
pub trait ConfigReader {
    /// Reads and constructs `Self` using the provided [`ConfigContext`].
    ///
    /// Returns `Some(Self)` on success, or `None` if any field failed to read or parse.
    fn read(ctx: &mut ConfigContext) -> Option<Self>
    where
        Self: Sized;
}

/// An error that occurred while reading or parsing a specific configuration key.
#[derive(Clone, Debug)]
pub struct ReadError {
    /// The full configuration key name that failed.
    pub key: String,
    /// The message explaining why reading or parsing failed.
    pub msg: ErrorMsg,
}

/// An aggregation of errors encountered when reading configuration.
#[derive(Clone, Debug)]
pub enum ReadErrors {
    /// Errors encountered while loading configuration sources (e.g. missing files or syntax errors).
    Source(Vec<SourceError>),
    /// Errors encountered while reading or parsing individual configuration keys.
    Read(Vec<ReadError>),
}

impl Display for ReadErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Config errors:")?;
        let sorted: BTreeSet<String> = match self {
            ReadErrors::Source(errs) => errs.iter().map(|e| format!("{}", e.0)).collect(),
            ReadErrors::Read(errs) => errs
                .iter()
                .map(|e| format!("{}: {}", e.key, e.msg.0))
                .collect(),
        };
        for s in sorted {
            write!(f, "\n  * {}", s)?;
        }
        Ok(())
    }
}

impl std::error::Error for ReadErrors {}

/// Execution context for reading configuration and generating usage reports.
///
/// Tracks loaded sources, records inspected keys and defaults, registers secret keys,
/// and accumulates read errors.
pub struct ConfigContext {
    /// The list of configuration sources, checked in order of priority.
    pub sources: Vec<ConfigSource>,
    secret_keys_lowercase: HashSet<String>,
    secret_keywords_lowercase: HashSet<String>,
    keys_seen: HashMap<String, Option<String>>, // key -> default
    read_errors: Vec<ReadError>,
    key_prefix: String,
}

impl Default for ConfigContext {
    fn default() -> Self {
        Self::new(vec![ConfigSource::env()])
    }
}

impl ConfigContext {
    /// Creates a new `ConfigContext` with the specified configuration sources.
    pub fn new(sources: Vec<ConfigSource>) -> Self {
        let mut secret_keywords_lowercase = HashSet::new();
        secret_keywords_lowercase.insert("password".to_string());
        secret_keywords_lowercase.insert("secret".to_string());
        ConfigContext {
            sources,
            secret_keys_lowercase: HashSet::new(),
            secret_keywords_lowercase,
            keys_seen: HashMap::new(),
            read_errors: Vec::new(),
            key_prefix: String::new(),
        }
    }

    /// Executes a function with a temporary key prefix prepended to all lookups.
    ///
    /// After the function finishes, the previous prefix state is restored.
    pub fn with_key_prefix<R>(&mut self, prefix: &str, f: impl FnOnce(&mut Self) -> R) -> R {
        if prefix.is_empty() {
            f(self)
        } else {
            let prev_len = self.key_prefix.len();
            self.key_prefix.push_str(prefix);
            let res = f(self);
            self.key_prefix.truncate(prev_len);
            res
        }
    }

    fn full_key(&self, key: &str) -> String {
        if self.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{key}", self.key_prefix)
        }
    }

    /// Marks a specific configuration key (case-insensitive) as secret, masking its value in reports.
    pub fn add_secret_key(&mut self, k: &str) {
        self.secret_keys_lowercase
            .insert(self.full_key(k).to_ascii_lowercase());
    }

    /// Marks any key containing the given keyword (case-insensitive) as secret.
    pub fn add_secret_keyword(&mut self, kw: &str) {
        self.secret_keywords_lowercase
            .insert(kw.to_ascii_lowercase());
    }

    /// Reads and populates a configuration structure implementing [`ConfigReader`].
    ///
    /// First checks all sources for loading errors; if any source errored, returns [`ReadErrors::Source`].
    /// Otherwise, calls [`ConfigReader::read`] and returns [`ReadErrors::Read`] on missing or invalid keys.
    pub fn read<A: ConfigReader>(&mut self) -> Result<A, ReadErrors> {
        // 1. Check all sources for errors
        let source_errors: Vec<SourceError> = self
            .sources
            .iter()
            .filter_map(|s| s.data.as_ref().err().cloned())
            .flatten()
            .collect();
        if !source_errors.is_empty() {
            return Err(ReadErrors::Source(source_errors));
        }

        // 2. Read config
        match A::read(self) {
            Some(a) if self.read_errors.is_empty() => Ok(a),
            _ => Err(ReadErrors::Read(self.read_errors.clone())),
        }
    }

    fn lookup<A: ConfigParser>(&mut self, key: &str, default: Option<String>) -> Option<Option<A>> {
        let full_key = self.full_key(key);
        self.keys_seen.insert(full_key.clone(), default);
        match self.sources.iter().find_map(|src| src.get(&full_key)) {
            Some(str) => match A::parse_config(str) {
                Ok(a) => Some(Some(a)),
                Err(msg) => {
                    self.read_errors.push(ReadError { key: full_key, msg });
                    None
                }
            },
            None => Some(None),
        }
    }

    /// Reads an optional configuration key.
    ///
    /// Returns `Some(Some(val))` if present and parsed successfully, `Some(None)` if not set,
    /// or `None` if parsing failed.
    pub fn get<A: ConfigParser>(&mut self, key: &str) -> Option<Option<A>> {
        self.lookup(key, Some("None".to_string()))
    }

    /// Reads a configuration key, falling back to the provided default value if absent.
    /// Returns `None` if parsing failed.
    pub fn get_or_use<A: ConfigParser + Debug>(&mut self, key: &str, default: A) -> Option<A> {
        match self.lookup(key, Some(format!("{:?}", default))) {
            Some(Some(a)) => Some(a),
            Some(None) => Some(default),
            None => None,
        }
    }

    /// Reads a configuration key, parsing the provided default string if the key is absent.
    /// Returns `None` if parsing failed.
    pub fn get_or_parse<A: ConfigParser>(&mut self, key: &str, default: &str) -> Option<A> {
        match self.lookup(key, Some(default.to_string())) {
            Some(Some(a)) => Some(a),
            Some(None) => match A::parse_config(default) {
                Ok(a) => Some(a),
                Err(msg) => {
                    self.read_errors.push(ReadError {
                        key: self.full_key(key),
                        msg,
                    });
                    None
                }
            },
            None => None,
        }
    }

    /// Reads a required configuration key, recording an error if absent or invalid.
    pub fn need<A: ConfigParser>(&mut self, key: &str) -> Option<A> {
        match self.lookup(key, None) {
            Some(Some(a)) => Some(a),
            Some(None) => {
                self.read_errors.push(ReadError {
                    key: self.full_key(key),
                    msg: ErrorMsg(String::from("not specified")),
                });
                None
            }
            None => None,
        }
    }

    /// Generates a formatted ASCII table showing all inspected keys, their values across sources,
    /// and any defaults applied, with sensitive values obfuscated.
    pub fn report_used(&self) -> String {
        use ascii_table_rs::{AsciiTable, CellValue};

        let mut table = AsciiTable::new("Config Report");
        {
            let mut headers = Vec::new();
            headers.push("Key");
            for s in &self.sources {
                headers.push(s.name.as_str());
            }
            headers.push("Default");
            table.set_headers(headers);
        }

        let all_keys: BTreeSet<&String> = self.keys_seen.keys().collect();

        fn push_value(
            ctx: &ConfigContext,
            row: &mut Vec<CellValue>,
            key_lowercase: &String,
            value: Option<String>,
        ) {
            let secret = ctx.secret_keys_lowercase.contains(key_lowercase)
                || ctx
                    .secret_keywords_lowercase
                    .iter()
                    .any(|kw| key_lowercase.contains(kw));
            let v = match value {
                Some(v) if secret => {
                    let hash = fnv1a_hash(&v);
                    format!("Obfuscated ({hash:08X})")
                }
                Some(v) if v.is_empty() => "\"\"".to_string(),
                Some(v) => v,
                None => "".to_string(),
            };
            row.push(CellValue::Str(v));
        }

        for key in all_keys {
            let key_lowercase = key.to_ascii_lowercase();
            let mut row = Vec::new();

            row.push(CellValue::Str(key.to_string()));

            for s in &self.sources {
                let value = s.get(key).cloned();
                push_value(self, &mut row, &key_lowercase, value);
            }

            let default = self.keys_seen.get(key).unwrap().clone();
            push_value(self, &mut row, &key_lowercase, default);

            table.add_row(row);
        }

        table.render_to_string().trim().to_string()
    }
}

fn fnv1a_hash(s: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5; // 32-bit FNV offset basis
    for &byte in s.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193); // 32-bit FNV prime
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_source_errors() {
        let errors = ReadErrors::Source(vec![
            SourceError(ErrorMsg("failed to load x".to_string())),
            SourceError(ErrorMsg("failed to load a".to_string())),
        ]);
        let actual = format!("{}", errors);
        let expect = "Config errors:\n  * failed to load a\n  * failed to load x";
        assert_eq!(actual.as_str(), expect);
    }

    #[test]
    fn display_read_errors() {
        let errors = ReadErrors::Read(vec![
            ReadError {
                key: "X".to_string(),
                msg: ErrorMsg("not specified".to_string()),
            },
            ReadError {
                key: "A".to_string(),
                msg: ErrorMsg("not specified".to_string()),
            },
        ]);
        let actual = format!("{}", errors);
        let expect = "Config errors:\n  * A: not specified\n  * X: not specified";
        assert_eq!(actual.as_str(), expect);
    }

    #[test]
    fn report_used() {
        let mut data1 = HashMap::new();
        data1.insert("A".to_string(), "val_a".to_string());
        data1.insert("AB".to_string(), "".to_string());
        data1.insert("DB_PASSWORD".to_string(), "abc".to_string());

        let mut data2 = HashMap::new();
        data2.insert("AB".to_string(), "from_src2".to_string());
        data2.insert("C".to_string(), "val_c".to_string());

        let mut ctx = ConfigContext::new(vec![
            ConfigSource {
                name: "Src1".to_string(),
                data: Ok(data1),
            },
            ConfigSource {
                name: "Src2".to_string(),
                data: Ok(data2),
            },
        ]);

        let _ = ctx.need::<String>("A");
        let _ = ctx.get::<String>("AB");
        let _ = ctx.get_or_parse::<u16>("C", "9000");
        let _ = ctx.get_or_parse::<String>("D", "");
        let _ = ctx.get_or_use::<bool>("E", true);
        let _ = ctx.get_or_parse::<String>("DB_PASSWORD", "def");
        ctx.add_secret_key("a");

        let actual = ctx.report_used();
        let expect = r#"
╭─────────────────────────────────────────────────────────────────────────╮
│                              Config Report                              │
├─────────────┬───────────────────────┬───────────┬───────────────────────┤
│ Key         │ Src1                  │ Src2      │ Default               │
├─────────────┼───────────────────────┼───────────┼───────────────────────┤
│ A           │ Obfuscated (5A9FA0E8) │           │                       │
│ AB          │ ""                    │ from_src2 │ None                  │
│ C           │                       │ val_c     │ 9000                  │
│ D           │                       │           │ ""                    │
│ DB_PASSWORD │ Obfuscated (1A47E90B) │           │ Obfuscated (C5597E8C) │
│ E           │                       │           │ true                  │
╰─────────────┴───────────────────────┴───────────┴───────────────────────╯
"#
        .trim();
        if actual != expect {
            println!("{}", actual);
        }
        assert_eq!(actual, expect);
    }
}

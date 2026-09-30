extern crate self as clear_config;

mod parsing;
mod source;

pub use crate::parsing::*;
pub use crate::source::*;
pub use clear_config_derive::{ConfigParser, ConfigReader};

use std::collections::{BTreeSet, HashMap};
use std::fmt::{Debug, Display};

pub trait ConfigReader {
    fn read(ctx: &mut ConfigContext) -> Option<Self>
    where
        Self: Sized;
}

#[derive(Clone, Debug)]
pub struct LoadError {
    pub key: String,
    pub msg: ErrorMsg,
}

#[derive(Clone, Debug)]
pub struct LoadErrors(Vec<LoadError>);

impl Display for LoadErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Config errors:").unwrap();
        let sorted: BTreeSet<String> = self
            .0
            .iter()
            .map(|e| format!("{}: {}", e.key, e.msg.0))
            .collect();
        for s in sorted {
            write!(f, "\n  * {}", s).unwrap();
        }
        Ok(())
    }
}

pub struct ConfigContext {
    pub sources: Vec<ConfigSource>,
    keys_seen: HashMap<String, Option<String>>, // key -> default
    load_errors: Vec<LoadError>,
}

impl Default for ConfigContext {
    fn default() -> Self {
        Self::new(vec![ConfigSource::env()])
    }
}

impl ConfigContext {
    pub fn new(sources: Vec<ConfigSource>) -> Self {
        ConfigContext {
            sources,
            keys_seen: HashMap::new(),
            load_errors: Vec::new(),
        }
    }

    pub fn load<A: ConfigReader>(&mut self) -> Result<A, LoadErrors> {
        match A::read(self) {
            Some(a) if self.load_errors.is_empty() => Ok(a),
            _ => Err(LoadErrors(self.load_errors.clone())),
        }
    }

    fn lookup<A: ConfigParser>(
        &mut self,
        key: &String,
        default: Option<String>,
    ) -> Option<Option<A>> {
        self.keys_seen.insert(key.clone(), default);
        match self.sources.iter().find_map(|src| src.get(key)) {
            Some(str) => match A::parse_config(str) {
                Ok(a) => Some(Some(a)),
                Err(msg) => {
                    self.load_errors.push(LoadError {
                        key: key.clone(),
                        msg,
                    });
                    None
                }
            },
            None => Some(None),
        }
    }

    pub fn get<A: ConfigParser>(&mut self, key: &String) -> Option<Option<A>> {
        self.lookup(key, None)
    }

    pub fn get_or_use<A: ConfigParser + Debug>(&mut self, key: &String, default: A) -> Option<A> {
        match self.lookup(key, Some(format!("{:?}", default))) {
            Some(Some(a)) => Some(a),
            Some(None) => Some(default),
            None => None,
        }
    }

    pub fn get_or_parse<A: ConfigParser>(&mut self, key: &String, default: &str) -> Option<A> {
        match self.lookup(key, Some(default.to_string())) {
            Some(Some(a)) => Some(a),
            Some(None) => match A::parse_config(default) {
                Ok(a) => Some(a),
                Err(msg) => {
                    self.load_errors.push(LoadError {
                        key: key.clone(),
                        msg,
                    });
                    None
                }
            },
            None => None,
        }
    }

    pub fn need<A: ConfigParser>(&mut self, key: &String) -> Option<A> {
        match self.lookup(key, None) {
            Some(Some(a)) => Some(a),
            Some(None) => {
                self.load_errors.push(LoadError {
                    key: key.clone(),
                    msg: ErrorMsg(String::from("not specified")),
                });
                None
            }
            None => None,
        }
    }

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

        for key in all_keys {
            let mut row = Vec::new();

            row.push(CellValue::Str(key.to_string()));

            for s in &self.sources {
                let value = s.get(key).cloned().unwrap_or_default();
                row.push(CellValue::Str(value));
            }

            let default = self.keys_seen.get(key).unwrap().clone().unwrap_or_default();
            row.push(CellValue::Str(default));

            table.add_row(row);
        }

        table.render_to_string().trim().to_string()
    }
}

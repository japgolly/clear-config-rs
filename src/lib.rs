extern crate self as clear_config;

mod parsing;

use std::collections::HashMap;
use std::fmt::Debug;

pub use crate::parsing::*;
pub use clear_config_derive::ConfigParser;

pub trait ConfigDef {
    fn load(ctx: &mut ConfigContext) -> Option<Self>
    where
        Self: Sized;
}

#[derive(Clone, Debug)]
pub struct LoadError {
    pub key: String,
    pub msg: ErrorMsg,
}

pub struct ConfigSource {
    pub name: String,
    pub data: HashMap<String, String>,
}

impl ConfigSource {
    pub fn env() -> Self {
        ConfigSource {
            name: String::from("Environment"),
            data: std::env::vars().collect(),
        }
    }

    pub fn get(&self, key: &String) -> Option<&String> {
        self.data.get(key)
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

    pub fn load<A: ConfigDef>(&mut self) -> Result<A, Vec<LoadError>> {
        match A::load(self) {
            Some(a) if self.load_errors.is_empty() => Ok(a),
            _ => Err(self.load_errors.clone()),
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
}

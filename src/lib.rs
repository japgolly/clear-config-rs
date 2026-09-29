mod parsing;

use std::collections::HashMap;

use crate::parsing::*;

pub trait ConfigDef {
    fn load(ctx: &mut ConfigContext) -> Result<Self, Vec<LoadError>>
    where
        Self: Sized;
}

#[derive(Debug)]
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
        }
    }

    pub fn need<A: ConfigParser>(&mut self, key: &String) -> Result<A, LoadError> {
        self.keys_seen.insert(key.clone(), None);
        match self.sources.iter().find_map(|src| src.get(key)) {
            Some(str) => A::parse_config(str).map_err(|msg| LoadError {
                key: key.clone(),
                msg,
            }),
            None => Err(LoadError {
                key: key.clone(),
                msg: ErrorMsg(String::from("not specified")),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}

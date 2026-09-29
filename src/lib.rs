use std::collections::HashMap;

pub trait ConfigDef {
    fn load(ctx: &mut ConfigContext) -> Result<Self, Vec<ErrorMsg>>
    where
        Self: Sized;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ErrorMsg {
    pub key: String,
    pub msg: String,
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

    pub fn need_string(&mut self, key: &String) -> Result<&String, ErrorMsg> {
        self.keys_seen.insert(key.clone(), None);
        let v = self.sources.iter().find_map(|src| src.get(key));
        v.ok_or_else(|| ErrorMsg {
            key: key.clone(),
            msg: String::from("not specified"),
        })
    }

    pub fn need_bool(&mut self, key: &String) -> Result<bool, ErrorMsg> {
        let s = self.need_string(key)?;
        // todo: use regex
        s.parse::<bool>().map_err(|_| ErrorMsg {
            key: key.clone(),
            msg: format!("{s:?} is not a valid bool"),
        })
    }

    pub fn need_u16(&mut self, key: &String) -> Result<u16, ErrorMsg> {
        let s = self.need_string(key)?;
        s.parse::<u16>().map_err(|_| ErrorMsg {
            key: key.clone(),
            msg: format!("{s:?} is not a valid u16"),
        })
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

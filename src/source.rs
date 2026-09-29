use std::collections::HashMap;

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

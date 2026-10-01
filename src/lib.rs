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
pub struct ReadError {
    pub key: String,
    pub msg: ErrorMsg,
}

#[derive(Clone, Debug)]
pub enum ReadErrors {
    Source(Vec<SourceError>),
    Read(Vec<ReadError>),
}

impl Display for ReadErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Config errors:").unwrap();
        let sorted: BTreeSet<String> = match self {
            ReadErrors::Source(errs) => errs.iter().map(|e| format!("{}", e.0)).collect(),
            ReadErrors::Read(errs) => errs
                .iter()
                .map(|e| format!("{}: {}", e.key, e.msg.0))
                .collect(),
        };
        for s in sorted {
            write!(f, "\n  * {}", s).unwrap();
        }
        Ok(())
    }
}

impl std::error::Error for ReadErrors {}

pub struct ConfigContext {
    pub sources: Vec<ConfigSource>,
    keys_seen: HashMap<String, Option<String>>, // key -> default
    read_errors: Vec<ReadError>,
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
            read_errors: Vec::new(),
        }
    }

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
                    self.read_errors.push(ReadError {
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
        self.lookup(key, Some("None".to_string()))
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
                    self.read_errors.push(ReadError {
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
                self.read_errors.push(ReadError {
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
}

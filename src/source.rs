use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

use crate::ErrorMsg;

#[derive(Clone, Debug, PartialEq)]
pub struct SourceError(pub ErrorMsg);

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SourceError {}

#[derive(Debug, PartialEq)]
pub struct ConfigSource {
    pub name: String,
    pub data: Result<HashMap<String, String>, Vec<SourceError>>,
}

impl ConfigSource {
    pub fn env() -> Self {
        ConfigSource {
            name: String::from("Environment"),
            data: Ok(std::env::vars().collect()),
        }
    }

    pub fn env_file(filename: impl AsRef<Path>, mandatory: bool) -> Self {
        match filename.as_ref().to_str() {
            None => {
                let err_msg = ErrorMsg("Invalid filename".to_string());
                ConfigSource {
                    name: "?".to_string(),
                    data: Err(vec![SourceError(err_msg)]),
                }
            }
            Some(name) => match std::fs::read_to_string(name) {
                Ok(content) => Self::env_file_content(name, content.as_str()),
                Err(e) if e.kind() == ErrorKind::NotFound && !mandatory => {
                    Self::env_file_content(name, "")
                }
                Err(e) => {
                    let err_msg = ErrorMsg(format!("{}: {}", name, e));
                    ConfigSource {
                        name: name.to_string(),
                        data: Err(vec![SourceError(err_msg)]),
                    }
                }
            },
        }
    }

    pub fn env_file_content(name: &str, content: &str) -> Self {
        let mut errors = Vec::new();
        let mut map = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let i = line.find('=');
            match i {
                Some(0) | None => {
                    errors.push(SourceError(ErrorMsg(
                        format!("{}: Invalid line: {}", name, line).to_string(),
                    )));
                    continue;
                }
                _ => (),
            };
            let i = i.unwrap();
            let name = line[..i].trim();
            let value = line[i + 1..].trim();
            map.insert(name.to_string(), value.to_string());
        }
        let data = if errors.is_empty() {
            Ok(map)
        } else {
            Err(errors)
        };
        ConfigSource {
            name: name.to_string(),
            data,
        }
    }

    pub fn get(&self, key: &String) -> Option<&String> {
        self.data.as_ref().ok().and_then(|m| m.get(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_file_not_found_optional() {
        let src = ConfigSource::env_file(".env.missing", false);
        assert_eq!(
            src,
            ConfigSource {
                name: ".env.missing".to_string(),
                data: Ok(HashMap::new()),
            }
        )
    }

    #[test]
    fn env_file_not_found_mandatory() {
        let src = ConfigSource::env_file(".env.missing", true);
        assert_eq!(
            src,
            ConfigSource {
                name: ".env.missing".to_string(),
                data: Err(vec![SourceError(ErrorMsg(
                    ".env.missing: No such file or directory (os error 2)".to_string()
                ))])
            }
        )
    }

    #[test]
    fn env_file_content_ok() {
        let content = r"
# Comment 1
    # Comment 2

X=abc
Y_Y=def
Z=
T = trimmed
        ";
        let mut expect = HashMap::new();
        expect.insert("X".to_string(), "abc".to_string());
        expect.insert("Y_Y".to_string(), "def".to_string());
        expect.insert("Z".to_string(), "".to_string());
        expect.insert("T".to_string(), "trimmed".to_string());
        let name = ".env";
        let src = ConfigSource::env_file_content(name, content);
        assert_eq!(
            src,
            ConfigSource {
                name: name.to_string(),
                data: Ok(expect)
            }
        )
    }

    #[test]
    fn env_file_content_ko() {
        let content = r"
# Comment

X=abc
Y_Y
=def

        ";
        let name = ".env";
        let src = ConfigSource::env_file_content(name, content);
        assert_eq!(
            src,
            ConfigSource {
                name: name.to_string(),
                data: Err(vec![
                    SourceError(ErrorMsg(".env: Invalid line: Y_Y".to_string())),
                    SourceError(ErrorMsg(".env: Invalid line: =def".to_string())),
                ])
            }
        )
    }
}

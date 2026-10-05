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

            let key = line[..i].trim();
            let key = key.strip_prefix("export ").unwrap_or(key).trim();
            if key.chars().any(|c| c.is_whitespace()) {
                errors.push(SourceError(ErrorMsg(
                    format!("{}: Invalid line: {}", name, line).to_string(),
                )));
                continue;
            }

            let raw_value = line[i + 1..].trim();
            let mut quote_char = None;
            let mut whitespace = String::new();
            let mut value = String::new();
            for c in raw_value.chars() {
                match quote_char {
                    None => {
                        if c == '#' {
                            break;
                        };
                        if c.is_whitespace() {
                            whitespace.push(c);
                            continue;
                        };
                        if !whitespace.is_empty() {
                            value.push_str(&whitespace);
                            whitespace.clear();
                        }
                        if c == '"' || c == '\'' {
                            quote_char = Some(c);
                            continue;
                        };
                    }
                    Some(q) => {
                        if c == q {
                            quote_char = None;
                            continue;
                        };
                    }
                };
                value.push(c);
            }
            if quote_char.is_some() {
                errors.push(SourceError(ErrorMsg(
                    format!("{}: Invalid line: {}", name, line).to_string(),
                )));
                continue;
            }

            map.insert(key.to_string(), value.to_string());
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

    pub fn in_memory<K, V, I>(name: &str, entries: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, V)>,
    {
        ConfigSource {
            name: name.to_string(),
            data: Ok(entries
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect()),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
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
        let content = r#"
# Comment 1
    # Comment 2

X1=abc #inline comment
X2 = abc def#inline comment
    export Y_Y='de#f' #inline comment
Z=
W1='  ' #whitespace
W2='  ' x #whitespace
T = "trimmed"
        "#;
        let mut expect = HashMap::new();
        expect.insert("X1".to_string(), "abc".to_string());
        expect.insert("X2".to_string(), "abc def".to_string());
        expect.insert("Y_Y".to_string(), "de#f".to_string());
        expect.insert("Z".to_string(), "".to_string());
        expect.insert("W1".to_string(), "  ".to_string());
        expect.insert("W2".to_string(), "   x".to_string());
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

Q='unterminated
X=abc
Y_Y
Z Z=invalid
=def

        ";
        let name = ".env";
        let src = ConfigSource::env_file_content(name, content);
        assert_eq!(
            src,
            ConfigSource {
                name: name.to_string(),
                data: Err(vec![
                    SourceError(ErrorMsg(".env: Invalid line: Q='unterminated".to_string())),
                    SourceError(ErrorMsg(".env: Invalid line: Y_Y".to_string())),
                    SourceError(ErrorMsg(".env: Invalid line: Z Z=invalid".to_string())),
                    SourceError(ErrorMsg(".env: Invalid line: =def".to_string())),
                ])
            }
        )
    }
}

use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, PartialEq)]
pub struct ErrorMsg(pub String);

pub trait ConfigParser: Sized {
    fn parse_config(s: &str) -> Result<Self, ErrorMsg>;
}

impl ConfigParser for String {
    fn parse_config(s: &str) -> Result<Self, ErrorMsg> {
        Ok(s.to_string())
    }
}

impl ConfigParser for bool {
    fn parse_config(s: &str) -> Result<Self, ErrorMsg> {
        static REGEX_TRUE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^(?:t(?:rue)?|y(?:es)?|1|on|enabled?)$").unwrap());
        static REGEX_FALSE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^(?:f(?:alse)?|n(?:o)?|0|off|disabled?)$").unwrap());

        if REGEX_TRUE.is_match(s) {
            Ok(true)
        } else if REGEX_FALSE.is_match(s) {
            Ok(false)
        } else {
            Err(ErrorMsg(format!("{s:?} is not a valid bool")))
        }
    }
}

macro_rules! impl_via_str_parse {
    ($($t:ty),*) => {
        $(
            impl ConfigParser for $t {
                fn parse_config(s: &str) -> Result<Self, ErrorMsg> {
                    s.parse::<$t>().map_err(|_| ErrorMsg(format!("{s:?} is not a valid {}", stringify!($t))))
                }
            }
        )*
    };
}

impl_via_str_parse!(u8, u16, u32, u64, u128, usize);
impl_via_str_parse!(i8, i16, i32, i64, i128, isize);
impl_via_str_parse!(f32, f64, char);

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    fn assert_parses<A: ConfigParser + PartialEq + Debug>(str: &str, expect: A) {
        let actual = A::parse_config(str);
        assert_eq!(actual, Ok(expect));
    }

    #[test]
    fn test_bool_true() {
        assert_parses("1", true);
        assert_parses("Y", true);
        assert_parses("Yes", true);
        assert_parses("ON", true);
        assert_parses("T", true);
        assert_parses("true", true);
        assert_parses("enabled", true);
    }

    #[test]
    fn test_bool_false() {
        assert_parses("0", false);
        assert_parses("N", false);
        assert_parses("No", false);
        assert_parses("OFF", false);
        assert_parses("F", false);
        assert_parses("false", false);
        assert_parses("disabled", false);
    }

    #[test]
    fn test_u8() {
        assert_parses::<u8>("123", 123);
    }

    #[test]
    fn test_u16() {
        assert_parses::<u16>("123", 123);
    }
}

use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug)]
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

macro_rules! impl_via_parse {
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

impl_via_parse!(u8, u16, u32, u64, u128, usize);
impl_via_parse!(i8, i16, i32, i64, i128, isize);
impl_via_parse!(f32, f64, char);

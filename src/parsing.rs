use regex::Regex;
use std::sync::LazyLock;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct ErrorMsg(pub String);

impl std::fmt::Display for ErrorMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ErrorMsg {}

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

impl ConfigParser for Duration {
    fn parse_config(s: &str) -> Result<Self, ErrorMsg> {
        static REGEX_MAIN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(\d+)\s*([a-zA-Z]+)\s*$").unwrap());
        static REGEX_NANO: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^n(?:ano(?:sec(?:ond)?)?)?s?$").unwrap());
        static REGEX_MICRO: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^(?:u|micro(?:sec(?:ond)?)?)s?$").unwrap());
        static REGEX_MILLI: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^milli(?:sec(?:ond)?)?s?|ms$").unwrap());
        static REGEX_SEC: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^s(?:ec(?:onds?)?)?$").unwrap());
        static REGEX_MIN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^m(?:in(?:ute)?)?s?$").unwrap());
        static REGEX_HR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^h(?:(?:ou)?r)?s?$").unwrap());

        match REGEX_MAIN.captures(s) {
            None => Err(ErrorMsg(format!("{s:?} is not a valid duration"))),
            Some(m) => {
                let qty = m.get(1).unwrap().as_str();
                match qty.parse::<u64>() {
                    Err(_) => Err(ErrorMsg(format!("{qty:?} is not a valid u64"))),
                    Ok(qty) => {
                        let units = m.get(2).unwrap().as_str();
                        if REGEX_NANO.is_match(units) {
                            Ok(Duration::from_nanos(qty))
                        } else if REGEX_MICRO.is_match(units) {
                            Ok(Duration::from_micros(qty))
                        } else if REGEX_MILLI.is_match(units) {
                            Ok(Duration::from_millis(qty))
                        } else if REGEX_SEC.is_match(units) {
                            Ok(Duration::from_secs(qty))
                        } else if REGEX_MIN.is_match(units) {
                            Ok(Duration::from_mins(qty))
                        } else if REGEX_HR.is_match(units) {
                            Ok(Duration::from_hours(qty))
                        } else {
                            Err(ErrorMsg(format!("{units:?} is not a valid time unit")))
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clear_config_derive::ConfigParser;
    use std::fmt::Debug;

    #[derive(ConfigParser, PartialEq, Debug)]
    enum Env {
        Dev,
        Prod,
    }

    fn assert_parses<A: ConfigParser + PartialEq + Debug>(str: &str, expect: A) {
        let actual = A::parse_config(str);
        assert_eq!(actual, Ok(expect));
    }

    fn assert_parse_fails<A: ConfigParser + PartialEq + Debug>(str: &str, expect: &str) {
        let actual = A::parse_config(str);
        assert_eq!(actual, Err(ErrorMsg(expect.to_string())));
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

    #[test]
    fn test_enum_ok() {
        assert_parses("dev", Env::Dev);
        assert_parses("Dev", Env::Dev);
        assert_parses("prod", Env::Prod);
        assert_parses("PROD", Env::Prod);
    }

    #[test]
    fn test_enum_ko() {
        assert_parse_fails::<Env>(
            "what",
            "\"what\" is not a valid Env (expected one of: Dev, Prod)",
        );
    }

    #[test]
    fn test_duration_nanos() {
        assert_parses("30n", Duration::from_nanos(30));
        assert_parses("30ns", Duration::from_nanos(30));
        assert_parses("30 nano", Duration::from_nanos(30));
        assert_parses("30 nanos", Duration::from_nanos(30));
        assert_parses("30 nanosec", Duration::from_nanos(30));
        assert_parses("30 nanosecs", Duration::from_nanos(30));
        assert_parses("30 nanosecond", Duration::from_nanos(30));
        assert_parses("30 nanoseconds", Duration::from_nanos(30));
    }

    #[test]
    fn test_duration_micro() {
        assert_parses("30u", Duration::from_micros(30));
        assert_parses("30us", Duration::from_micros(30));
        assert_parses("30 micro", Duration::from_micros(30));
        assert_parses("30 micros", Duration::from_micros(30));
        assert_parses("30 microsec", Duration::from_micros(30));
        assert_parses("30 microsecs", Duration::from_micros(30));
        assert_parses("30 microsecond", Duration::from_micros(30));
        assert_parses("30 microseconds", Duration::from_micros(30));
    }

    #[test]
    fn test_duration_millis() {
        assert_parses("30ms", Duration::from_millis(30));
        assert_parses("30 milli", Duration::from_millis(30));
        assert_parses("30 millis", Duration::from_millis(30));
        assert_parses("30 millisec", Duration::from_millis(30));
        assert_parses("30 millisecs", Duration::from_millis(30));
        assert_parses("30 millisecond", Duration::from_millis(30));
        assert_parses("30 milliseconds", Duration::from_millis(30));
    }

    #[test]
    fn test_duration_sec() {
        assert_parses("30s", Duration::from_secs(30));
        assert_parses(" 90 sec ", Duration::from_secs(90));
        assert_parses("60 Seconds", Duration::from_secs(60));
    }

    #[test]
    fn test_duration_min() {
        assert_parses("10m", Duration::from_mins(10));
        assert_parses("10 min", Duration::from_mins(10));
        assert_parses("10 mins", Duration::from_mins(10));
        assert_parses("10 minute", Duration::from_mins(10));
        assert_parses("10 minutes", Duration::from_mins(10));
    }

    #[test]
    fn test_duration_hour() {
        assert_parses("1h", Duration::from_hours(1));
        assert_parses("1 hr", Duration::from_hours(1));
        assert_parses("1 hrs", Duration::from_hours(1));
        assert_parses("1 hour", Duration::from_hours(1));
        assert_parses("1 hours", Duration::from_hours(1));
    }

    #[test]
    fn test_duration_bad_unit() {
        assert_parse_fails::<Duration>("30 spears", "\"spears\" is not a valid time unit");
    }
}

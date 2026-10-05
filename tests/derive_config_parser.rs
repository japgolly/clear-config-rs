use clear_config::{ConfigParser, ErrorMsg};

#[derive(ConfigParser, Debug, PartialEq)]
pub struct S<A>(pub A);

#[derive(ConfigParser, Debug, PartialEq)]
pub struct Port(pub u16);

#[test]
fn test_single_anonymous_field_generic() {
    assert_eq!(S::<u32>::parse_config("42"), Ok(S(42)));
    assert_eq!(
        S::<String>::parse_config("hello"),
        Ok(S("hello".to_string()))
    );
    assert_eq!(S::<bool>::parse_config("true"), Ok(S(true)));
    assert_eq!(
        S::<u32>::parse_config("invalid"),
        Err(ErrorMsg("\"invalid\" is not a valid u32".to_string()))
    );
}

#[test]
fn test_single_anonymous_field_concrete() {
    assert_eq!(Port::parse_config("8080"), Ok(Port(8080)));
    assert_eq!(
        Port::parse_config("not_a_number"),
        Err(ErrorMsg("\"not_a_number\" is not a valid u16".to_string()))
    );
}

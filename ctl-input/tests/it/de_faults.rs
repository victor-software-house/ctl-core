//! Each problem type with the bytes it sits at.

use ctl_input::Limits;
use ctl_input::de::deserialize;
use ctl_input::format::{Fault, Format, Reading, TOML, YAML};
use serde::Deserialize;

use crate::common::{at, refused, typed};

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct Doc {
    #[expect(dead_code, reason = "only the refusal is read")]
    server: Server,
}

#[derive(Debug, Deserialize)]
struct Fleet {
    #[expect(dead_code, reason = "only the refusal is read")]
    servers: Vec<Server>,
}

#[test]
fn an_unknown_field_sits_on_its_key() {
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        #[expect(dead_code, reason = "only the refusal is read")]
        name: String,
    }
    let text = "name: a\nextra: 1\n";
    let refusal = refused::<Strict>(&YAML, text);
    assert_eq!(
        refusal.fault(),
        &Fault::UnknownField {
            field: "extra".to_owned(),
            expected: vec!["name"]
        }
    );
    assert_eq!(at(text, &refusal), "extra");
    let text = "name = 'a'\nextra = 1\n";
    let refusal = refused::<Strict>(&TOML, text);
    assert_eq!(at(text, &refusal), "extra");
}

#[test]
fn a_missing_field_sits_on_the_key_that_holds_its_mapping() {
    let text = "title: t\nserver:\n  host: h\n";
    let refusal = refused::<Doc>(&YAML, text);
    assert_eq!(refusal.fault(), &Fault::MissingField { field: "port" });
    assert_eq!(at(text, &refusal), "server");
    assert_eq!(refusal.span().map(|span| span.start), Some(9));
    assert_eq!(refusal.defined_at(), None);
    let text = "[server]\nhost = 'h'\n";
    let refusal = refused::<Doc>(&TOML, text);
    assert_eq!(refusal.fault(), &Fault::MissingField { field: "port" });
    assert_eq!(at(text, &refusal), "server");
    let text = "server = { host = 'h' }\n";
    let refusal = refused::<Doc>(&TOML, text);
    assert_eq!(at(text, &refusal), "server");
}

#[test]
fn a_missing_field_of_the_root_sits_at_the_start_of_the_text() {
    let refusal = refused::<Server>(&YAML, "host: h\n");
    assert_eq!(refusal.fault(), &Fault::MissingField { field: "port" });
    assert_eq!(refusal.span(), Some(0..0));
    let refusal = refused::<Server>(&TOML, "host = 'h'\n");
    assert_eq!(refusal.span(), Some(0..0));
}

#[test]
fn a_missing_field_of_a_sequence_item_sits_on_the_item() {
    let text = "servers:\n  - host: h\n    port: 1\n  - host: g\n";
    let refusal = refused::<Fleet>(&YAML, text);
    assert_eq!(refusal.fault(), &Fault::MissingField { field: "port" });
    assert_eq!(at(text, &refusal), "host: g");
}

#[test]
fn a_field_written_twice_in_a_listing_sits_on_the_second_key() {
    #[derive(Debug, Deserialize)]
    struct Pair {
        #[expect(dead_code, reason = "only the refusal is read")]
        a: u8,
    }
    let text = "a: 1\na: 2\n";
    let tree = YAML.read(text, &Limits::NONE, Reading::Listing).unwrap();
    let refusal = deserialize::<Pair>(&tree, &YAML).unwrap_err();
    assert_eq!(
        refusal.fault(),
        &Fault::DuplicateKey {
            key: "a".to_owned()
        }
    );
    assert_eq!(refusal.span().map(|span| span.start), Some(5));
}

#[test]
fn a_type_that_refuses_its_own_value_keeps_its_text_at_the_node() {
    #[derive(Debug, Deserialize)]
    #[serde(try_from = "u16")]
    struct Port;
    impl TryFrom<u16> for Port {
        type Error = String;

        fn try_from(port: u16) -> Result<Self, String> {
            if port == 0 {
                Err("port must not be 0".to_owned())
            } else {
                Ok(Self)
            }
        }
    }
    #[derive(Debug, Deserialize)]
    struct Listen {
        #[expect(dead_code, reason = "only the refusal is read")]
        port: Port,
    }
    let text = "name: x\nport: 0\n";
    let refusal = refused::<Listen>(&YAML, text);
    assert_eq!(
        refusal.fault(),
        &Fault::InvalidValue {
            detail: "port must not be 0".to_owned()
        }
    );
    assert_eq!(at(text, &refusal), "0");
    let text = "port = 0\n";
    let refusal = refused::<Listen>(&TOML, text);
    assert_eq!(at(text, &refusal), "0");
}

#[test]
fn a_bad_value_through_an_alias_sits_at_the_alias_and_names_its_definition() {
    #[derive(Debug, Deserialize)]
    struct Listen {
        #[expect(dead_code, reason = "only the refusal is read")]
        port: u16,
    }
    let text = "base: &port eighty\na: 1\nb: 2\nport: *port\n";
    let refusal = refused::<Listen>(&YAML, text);
    assert_eq!(
        refusal.fault(),
        &Fault::InvalidValue {
            detail: "expected an integer".to_owned()
        }
    );
    assert_eq!(at(text, &refusal), "*port");
    let defined = refusal.defined_at().unwrap();
    assert!(text[defined].contains("eighty"));
}

#[test]
fn a_fault_inside_a_replayed_mapping_sits_at_the_alias_that_brought_it() {
    let text = "base: &s {host: h, port: x}\nserver: *s\n";
    let refusal = refused::<Doc>(&YAML, text);
    assert_eq!(at(text, &refusal), "*s");
    let defined = refusal.defined_at().unwrap();
    assert_eq!(&text[defined], "x");
}

#[test]
fn a_value_the_visitor_counts_wrongly_is_an_invalid_value_at_its_sequence() {
    #[derive(Debug, Deserialize)]
    struct Pair {
        #[expect(dead_code, reason = "only the refusal is read")]
        pair: (u8, u8),
    }
    let text = "pair: [1]\n";
    let refusal = refused::<Pair>(&YAML, text);
    assert!(matches!(refusal.fault(), Fault::InvalidValue { .. }));
    assert_eq!(at(text, &refusal), "[1]");
    assert_eq!(
        typed::<Server>(&YAML, "host: h\nport: 1\n").unwrap().port,
        1
    );
}

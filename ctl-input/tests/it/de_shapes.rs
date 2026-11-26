//! The kind of node each shape of type needs, and the enums and wrappers
//! built from them.

use std::collections::BTreeMap;

use ctl_input::format::{Fault, Format, TOML, YAML};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::common::{at, refused, typed};

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Doc {
    server: Server,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Labels {
    labels: Vec<String>,
}

fn expected(expected: &str, found: &str) -> Fault {
    Fault::ExpectedKind {
        expected: expected.to_owned(),
        found: found.to_owned(),
    }
}

#[test]
fn a_struct_and_a_map_need_a_mapping() {
    let text = "- 1\n";
    let refusal = refused::<Server>(&YAML, text);
    assert_eq!(refusal.fault(), &expected("a mapping", "a sequence"));
    assert!(at(text, &refusal).starts_with("- 1"));
    let text = "server: 8080\n";
    let refusal = refused::<Doc>(&YAML, text);
    assert_eq!(refusal.fault(), &expected("a mapping", "a scalar"));
    assert_eq!(at(text, &refusal), "8080");
    let text = "server = 1\n";
    let refusal = refused::<Doc>(&TOML, text);
    assert_eq!(refusal.fault(), &expected("a mapping", "a scalar"));
    assert_eq!(at(text, &refusal), "1");
    let refusal = refused::<BTreeMap<String, String>>(&YAML, "[a]\n");
    assert_eq!(refusal.fault(), &expected("a mapping", "a sequence"));
}

#[test]
fn a_sequence_a_tuple_and_a_tuple_struct_need_a_sequence() {
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the refusal is read")]
    struct Pair(u8, u8);
    let text = "labels: {a: 1}\n";
    let refusal = refused::<Labels>(&YAML, text);
    assert_eq!(refusal.fault(), &expected("a sequence", "a mapping"));
    assert_eq!(at(text, &refusal), "{a: 1}");
    let refusal = refused::<Labels>(&YAML, "labels: x\n");
    assert_eq!(refusal.fault(), &expected("a sequence", "a scalar"));
    let refusal = refused::<Pair>(&YAML, "a: 1\n");
    assert_eq!(refusal.fault(), &expected("a sequence", "a mapping"));
}

#[test]
fn a_scalar_type_given_a_collection_reports_what_serde_expected() {
    #[derive(Debug, Deserialize)]
    struct Port {
        #[expect(dead_code, reason = "only the refusal is read")]
        port: u16,
    }
    let text = "port: [1]\n";
    let refusal = refused::<Port>(&YAML, text);
    assert_eq!(refusal.fault(), &expected("u16", "a sequence"));
    assert_eq!(at(text, &refusal), "[1]");
    let refusal = refused::<Port>(&TOML, "port = { a = 1 }\n");
    assert_eq!(refusal.fault(), &expected("u16", "a mapping"));
}

#[test]
fn a_tuple_must_match_the_length_of_its_sequence() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Pair {
        pair: (u8, u8),
    }
    assert_eq!(
        typed::<Pair>(&YAML, "pair: [1, 2]\n").unwrap(),
        Pair { pair: (1, 2) }
    );
    let text = "pair: [1]\n";
    let refusal = refused::<Pair>(&YAML, text);
    assert_eq!(
        refusal.fault(),
        &Fault::InvalidValue {
            detail: "expected a tuple of size 2, found 1 items".to_owned()
        }
    );
    assert_eq!(at(text, &refusal), "[1]");
    let refusal = refused::<Pair>(&TOML, "pair = [1, 2, 3]\n");
    assert_eq!(
        refusal.fault(),
        &Fault::InvalidValue {
            detail: "expected 2 items, found 3 items".to_owned()
        }
    );
}

#[derive(Debug, Deserialize, PartialEq)]
enum Shape {
    Empty,
    Num(u8),
    Pair(u8, u8),
    Rect { w: u8, h: u8 },
}

#[derive(Debug, Deserialize, PartialEq)]
struct Drawing {
    shape: Shape,
}

#[test]
fn an_enum_is_a_name_or_a_mapping_of_one_entry() {
    let read =
        |format: &'static dyn Format, text: &str| typed::<Drawing>(format, text).unwrap().shape;
    assert_eq!(read(&YAML, "shape: Empty\n"), Shape::Empty);
    assert_eq!(read(&YAML, "shape: {Num: 3}\n"), Shape::Num(3));
    assert_eq!(read(&YAML, "shape:\n  Pair: [1, 2]\n"), Shape::Pair(1, 2));
    assert_eq!(
        read(&YAML, "shape:\n  Rect: {w: 1, h: 2}\n"),
        Shape::Rect { w: 1, h: 2 }
    );
    assert_eq!(read(&TOML, "shape = 'Empty'\n"), Shape::Empty);
    assert_eq!(read(&TOML, "shape = { Num = 3 }\n"), Shape::Num(3));
    assert_eq!(
        read(&TOML, "[shape.Rect]\nw = 1\nh = 2\n"),
        Shape::Rect { w: 1, h: 2 }
    );
}

#[test]
fn a_variant_the_enum_lacks_is_an_unknown_variant_at_its_name() {
    let all = ["Empty", "Num", "Pair", "Rect"].to_vec();
    let text = "shape: Round\n";
    let refusal = refused::<Drawing>(&YAML, text);
    assert_eq!(
        refusal.fault(),
        &Fault::UnknownVariant {
            variant: "Round".to_owned(),
            expected: all.clone()
        }
    );
    assert_eq!(at(text, &refusal), "Round");
    let text = "shape: {Round: 1}\n";
    let refusal = refused::<Drawing>(&YAML, text);
    assert_eq!(at(text, &refusal), "Round");
    assert!(matches!(refusal.fault(), Fault::UnknownVariant { .. }));
}

#[test]
fn any_other_node_is_not_an_enum() {
    let what = "a variant name or a mapping with one entry";
    let refusal = refused::<Drawing>(&YAML, "shape: {Num: 1, Empty: ~}\n");
    assert_eq!(refusal.fault(), &expected(what, "a mapping with 2 entries"));
    let refusal = refused::<Drawing>(&YAML, "shape: [Empty]\n");
    assert_eq!(refusal.fault(), &expected(what, "a sequence"));
    let refusal = refused::<Drawing>(&YAML, "shape: {}\n");
    assert_eq!(refusal.fault(), &expected(what, "a mapping with 0 entries"));
}

#[test]
fn a_newtype_struct_is_read_as_what_it_wraps() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wrapped(u8);
    #[derive(Debug, Deserialize, PartialEq)]
    struct Holder {
        a: Wrapped,
    }
    assert_eq!(
        typed::<Holder>(&YAML, "a: 7\n").unwrap(),
        Holder { a: Wrapped(7) }
    );
}

#[test]
fn an_ignored_value_is_never_read() {
    let value =
        typed::<Server>(&YAML, "host: h\nport: 1\nextra: .inf\nmore: !!int nope\n").unwrap();
    assert_eq!(value.port, 1);
}

#[test]
fn a_flattened_map_and_an_untagged_enum_read_through_a_typeless_buffer() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Assets {
        name: String,
        #[serde(flatten)]
        targets: BTreeMap<String, Vec<String>>,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(untagged)]
    enum Setting {
        List(Vec<String>),
        Word(String),
    }
    let assets = typed::<Assets>(&YAML, "name: n\nlinux: [a]\nmac: [b, c]\n").unwrap();
    assert_eq!(assets.targets.len(), 2);
    let settings = typed::<BTreeMap<String, Setting>>(&YAML, "a: [x]\nb: y\n").unwrap();
    assert_eq!(settings["a"], Setting::List(vec!["x".to_owned()]));
    assert_eq!(settings["b"], Setting::Word("y".to_owned()));
    let refusal = refused::<BTreeMap<String, Setting>>(&YAML, "up: .inf\n");
    assert!(matches!(
        refusal.fault(),
        Fault::InvalidValue { detail } if detail.contains("non-finite")
    ));
}

#[test]
fn a_typeless_read_of_a_mapping_keeps_the_order_and_nesting() {
    let text = "z: 1\na: {c: [1, {d: 2}]}\n";
    let value = typed::<Value>(&YAML, text).unwrap();
    assert_eq!(value, json!({"z": 1, "a": {"c": [1, {"d": 2}]}}));
}

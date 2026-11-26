//! The Deserializer reads each scalar as the type a field needs, by the rules
//! of the format that read it.

use std::collections::BTreeMap;
use std::fmt;

use ctl_input::format::{Fault, TOML, YAML};
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};
use serde_json::{Value, json};

use crate::common::{at, refused, typed};

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Flag {
    enabled: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Text {
    a: String,
}

fn invalid(detail: &str) -> Fault {
    Fault::InvalidValue {
        detail: detail.to_owned(),
    }
}

#[test]
fn a_quoted_number_fills_a_number_field_in_yaml_and_not_in_toml() {
    assert_eq!(
        typed::<Server>(&YAML, "port: \"8080\"\n").unwrap(),
        Server { port: 8080 }
    );
    let text = "port = \"8080\"\n";
    let refusal = refused::<Server>(&TOML, text);
    assert_eq!(refusal.fault(), &invalid("expected an integer"));
    assert_eq!(at(text, &refusal), "\"8080\"");
    assert_eq!(refusal.span().map(|span| span.start), Some(7));
}

#[test]
fn only_the_yaml_1_2_spellings_fill_a_bool() {
    for text in ["enabled: true\n", "enabled: True\n", "enabled: 'TRUE'\n"] {
        assert_eq!(
            typed::<Flag>(&YAML, text).unwrap(),
            Flag { enabled: true },
            "{text}"
        );
    }
    let text = "enabled: yes\n";
    let refusal = refused::<Flag>(&YAML, text);
    assert_eq!(refusal.fault(), &invalid("expected true or false"));
    assert_eq!(at(text, &refusal), "yes");
}

#[test]
fn a_toml_string_never_fills_a_bool_and_a_toml_bool_does() {
    assert_eq!(
        typed::<Flag>(&TOML, "enabled = true\n").unwrap(),
        Flag { enabled: true }
    );
    let text = "enabled = \"true\"\n";
    let refusal = refused::<Flag>(&TOML, text);
    assert_eq!(refusal.fault(), &invalid("expected true or false"));
    assert_eq!(at(text, &refusal), "\"true\"");
}

#[derive(Debug, Deserialize, PartialEq)]
struct Widths {
    a: i8,
    b: i16,
    c: i32,
    d: i64,
    e: i128,
    f: u8,
    g: u16,
    h: u32,
    i: u64,
    j: u128,
}

#[test]
fn every_integer_width_reads_its_own_range_in_yaml() {
    let text = "\
a: -128
b: -32768
c: 2147483647
d: -9223372036854775808
e: -170141183460469231731687303715884105728
f: 255
g: 0x10
h: 4294967295
i: 18446744073709551615
j: 340282366920938463463374607431768211455
";
    assert_eq!(
        typed::<Widths>(&YAML, text).unwrap(),
        Widths {
            a: i8::MIN,
            b: i16::MIN,
            c: i32::MAX,
            d: i64::MIN,
            e: i128::MIN,
            f: u8::MAX,
            g: 16,
            h: u32::MAX,
            i: u64::MAX,
            j: u128::MAX,
        }
    );
}

#[test]
fn an_integer_out_of_range_is_refused_by_the_type_that_cannot_hold_it() {
    #[derive(Debug, Deserialize)]
    struct Small {
        #[expect(dead_code, reason = "only the refusal is read")]
        a: i8,
    }
    #[derive(Debug, Deserialize)]
    struct Unsigned {
        #[expect(dead_code, reason = "only the refusal is read")]
        a: u8,
    }
    let text = "a: 300\n";
    let refusal = refused::<Small>(&YAML, text);
    assert_eq!(
        refusal.fault(),
        &invalid("expected i8, found integer `300`")
    );
    assert_eq!(at(text, &refusal), "300");
    let refusal = refused::<Unsigned>(&YAML, "a: -1\n");
    assert_eq!(refusal.fault(), &invalid("expected u8, found integer `-1`"));
    let refusal = refused::<Unsigned>(&TOML, "a = 256\n");
    assert_eq!(
        refusal.fault(),
        &invalid("expected u8, found integer `256`")
    );
}

#[test]
fn toml_integers_read_in_every_radix_and_a_float_field_takes_an_integer() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Numbers {
        hex: u32,
        octal: i64,
        wide: f64,
        narrow: f32,
    }
    let text = "hex = 0xff\noctal = 0o17\nwide = 3\nnarrow = 1.5\n";
    assert_eq!(
        typed::<Numbers>(&TOML, text).unwrap(),
        Numbers {
            hex: 255,
            octal: 15,
            wide: 3.0,
            narrow: 1.5,
        }
    );
    let refusal = refused::<Numbers>(&TOML, "hex = 1\noctal = 1\nwide = \"3\"\nnarrow = 1\n");
    assert_eq!(refusal.fault(), &invalid("expected a float"));
}

#[test]
fn floats_read_as_f32_and_f64_and_infinity_is_a_float_but_not_a_typeless_value() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Floats {
        a: f64,
        b: f32,
        c: f64,
    }
    let floats = typed::<Floats>(&YAML, "a: 2\nb: 1.5\nc: .inf\n").unwrap();
    assert_eq!(
        floats,
        Floats {
            a: 2.0,
            b: 1.5,
            c: f64::INFINITY
        }
    );
    let text = "up: .inf\n";
    let refusal = refused::<Value>(&YAML, text);
    assert_eq!(
        refusal.fault(),
        &invalid("non-finite float `.inf` rejected by the configured policy")
    );
    assert_eq!(at(text, &refusal), ".inf");
}

#[test]
fn a_string_field_refuses_a_null_and_a_tagged_non_string_and_keeps_a_quoted_null() {
    let text = "a: ~\n";
    let refusal = refused::<Text>(&YAML, text);
    assert_eq!(refusal.fault(), &invalid("expected text"));
    assert_eq!(at(text, &refusal), "~");
    for (text, expected) in [
        ("a: '~'\n", "~"),
        ("a: !!str ~\n", "~"),
        ("a: 8080\n", "8080"),
        ("a: !!binary aGk=\n", "hi"),
        ("a: true\n", "true"),
    ] {
        assert_eq!(typed::<Text>(&YAML, text).unwrap().a, expected, "{text}");
    }
    for text in ["a: !!int 8080\n", "a: !!int ~\n"] {
        let refusal = refused::<Text>(&YAML, text);
        assert_eq!(refusal.fault(), &invalid("expected text"), "{text}");
    }
}

#[test]
fn a_toml_string_field_takes_only_a_toml_string() {
    assert_eq!(typed::<Text>(&TOML, "a = 'x'\n").unwrap().a, "x");
    let text = "a = 1\n";
    let refusal = refused::<Text>(&TOML, text);
    assert_eq!(refusal.fault(), &invalid("expected text"));
    assert_eq!(at(text, &refusal), "1");
}

#[test]
fn a_char_field_takes_one_character() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Letter {
        a: char,
    }
    assert_eq!(typed::<Letter>(&YAML, "a: x\n").unwrap(), Letter { a: 'x' });
    let refusal = refused::<Letter>(&YAML, "a: xy\n");
    assert!(matches!(refusal.fault(), Fault::InvalidValue { .. }));
}

#[test]
fn a_unit_field_takes_only_a_null() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Nothing {
        a: (),
    }
    assert_eq!(
        typed::<Nothing>(&YAML, "a: ~\n").unwrap(),
        Nothing { a: () }
    );
    let refusal = refused::<Nothing>(&YAML, "a: 1\n");
    assert_eq!(refusal.fault(), &invalid("expected null"));
}

#[derive(Debug)]
struct Bytes(Vec<u8>);

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Reader;
        impl Visitor<'_> for Reader {
            type Value = Bytes;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("bytes")
            }

            fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<Bytes, E> {
                Ok(Bytes(bytes.to_vec()))
            }
        }
        deserializer.deserialize_bytes(Reader)
    }
}

#[derive(Debug, Deserialize)]
struct Payload {
    a: Bytes,
}

#[test]
fn a_bytes_field_takes_the_text_as_bytes() {
    assert_eq!(
        typed::<Payload>(&YAML, "a: hé\n").unwrap().a.0,
        "hé".as_bytes()
    );
    assert_eq!(
        typed::<Payload>(&YAML, "a: !!binary aGk=\n").unwrap().a.0,
        b"hi"
    );
    assert_eq!(typed::<Payload>(&TOML, "a = 'x'\n").unwrap().a.0, b"x");
    let refusal = refused::<Payload>(&TOML, "a = 1\n");
    assert_eq!(refusal.fault(), &invalid("expected text"));
}

#[test]
fn an_option_is_none_for_a_null_and_some_for_any_other_node() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Maybe {
        a: Option<u8>,
        #[serde(default)]
        b: Option<Vec<u8>>,
        #[serde(default)]
        c: Option<String>,
    }
    let all = |a, b, c| Maybe { a, b, c };
    assert_eq!(
        typed::<Maybe>(&YAML, "a: ~\n").unwrap(),
        all(None, None, None)
    );
    assert_eq!(
        typed::<Maybe>(&YAML, "a:\n").unwrap(),
        all(None, None, None)
    );
    assert_eq!(
        typed::<Maybe>(&YAML, "a: 1\nb: [2]\nc: '~'\n").unwrap(),
        all(Some(1), Some(vec![2]), Some("~".to_owned()))
    );
    assert_eq!(
        typed::<Maybe>(&TOML, "a = 1\nb = [2]\n").unwrap(),
        all(Some(1), Some(vec![2]), None)
    );
}

#[test]
fn a_typeless_read_gives_each_scalar_the_type_its_format_resolves() {
    let yaml = "\
a: 1
b: [true, ~, 1.5, x, '7']
c: {d: 18446744073709551615, e: -9223372036854775808}
";
    assert_eq!(
        typed::<Value>(&YAML, yaml).unwrap(),
        json!({
            "a": 1,
            "b": [true, null, 1.5, "x", "7"],
            "c": {"d": 18_446_744_073_709_551_615_u64, "e": i64::MIN},
        })
    );
    let toml = "\
a = 1
b = 'x'
c = [1, 2]
d = 1979-05-27T07:32:00Z

[t]
e = true
";
    assert_eq!(
        typed::<Value>(&TOML, toml).unwrap(),
        json!({
            "a": 1,
            "b": "x",
            "c": [1, 2],
            "d": "1979-05-27T07:32:00Z",
            "t": {"e": true},
        })
    );
}

#[test]
fn a_map_key_is_read_by_the_same_rules_as_a_value() {
    let ports = typed::<BTreeMap<u16, String>>(&YAML, "80: http\n443: https\n").unwrap();
    assert_eq!(ports.get(&443).map(String::as_str), Some("https"));
    let names = typed::<BTreeMap<String, u8>>(&YAML, "1: 2\n'x y': 3\n").unwrap();
    assert_eq!(names.keys().collect::<Vec<_>>(), ["1", "x y"]);
}

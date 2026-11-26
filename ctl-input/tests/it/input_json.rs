//! A JSON text through `Input`: each problem on its line and column, in the
//! words every format shares.

use std::collections::BTreeMap;

use ctl_input::format::JSON;
use ctl_input::{FieldPath, Format, Input, InputError, Limits, Problem, TOML, Written, YAML};
use indoc::indoc;
use serde::Deserialize;
use serde::de::IgnoredAny;

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    port: u16,
}

fn json(text: &str) -> Input {
    Input::new_with_format("app.json", text, &JSON)
}

/// The one problem `error` holds, as line, column and message.
fn one(error: &InputError) -> (Option<usize>, Option<usize>, &str) {
    assert_eq!(error.problems.len(), 1, "{error}");
    let Problem {
        line,
        column,
        message,
        ..
    } = &error.problems[0];
    (*line, *column, message)
}

#[test]
fn a_config_file_reads_into_a_typed_value() {
    let server: Server = json("{\"port\": 8080}").parse().unwrap();
    assert_eq!(server, Server { port: 8080 });
}

#[test]
fn a_trailing_comma_is_one_problem_on_the_brace() {
    let error = json("{\"a\": 1,}").parse::<IgnoredAny>().unwrap_err();
    assert_eq!(
        one(&error),
        (Some(1), Some(9), "expected a key in double quotes")
    );
}

#[test]
fn a_leading_zero_is_one_problem_on_the_number() {
    let error = json("{\"port\": 08080}").parse::<Server>().unwrap_err();
    assert_eq!(
        one(&error),
        (Some(1), Some(10), "number `08080` is not valid JSON")
    );
}

#[test]
fn text_after_the_value_is_one_problem_on_the_second_value() {
    let error = json("{} {}").parse::<IgnoredAny>().unwrap_err();
    assert_eq!(one(&error), (Some(1), Some(4), "text after the value"));
}

#[test]
fn a_byte_order_mark_is_no_column() {
    let error = json("\u{feff}{\"port\": \"x\"}")
        .parse::<Server>()
        .unwrap_err();
    assert_eq!(one(&error), (Some(1), Some(10), "expected an integer"));
}

#[test]
fn a_number_keeps_its_text_for_a_field_that_holds_it() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Big {
        id: u128,
    }
    let big: Big = json("{\"id\": 18446744073709551616}").parse().unwrap();
    assert_eq!(
        big,
        Big {
            id: 18_446_744_073_709_551_616
        }
    );
    let error = json("{\"id\": 340282366920938463463374607431768211456}")
        .parse::<Big>()
        .unwrap_err();
    assert_eq!(one(&error), (Some(1), Some(8), "expected an integer"));
}

#[test]
fn a_quoted_number_does_not_fill_an_integer_field() {
    let error = json("{\"port\": \"8080\"}").parse::<Server>().unwrap_err();
    assert_eq!(one(&error), (Some(1), Some(10), "expected an integer"));
}

#[test]
fn a_number_does_not_fill_a_string_field() {
    let error = json("{\"name\": 8080}")
        .parse::<BTreeMap<String, String>>()
        .unwrap_err();
    assert_eq!(one(&error), (Some(1), Some(10), "expected text"));
}

#[test]
fn null_reads_as_none_for_an_optional_field() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Named {
        name: Option<String>,
        port: Option<u16>,
    }
    let named: Named = json("{\"name\": null, \"port\": 80}").parse().unwrap();
    assert_eq!(
        named,
        Named {
            name: None,
            port: Some(80)
        }
    );
}

#[test]
fn null_does_not_fill_a_required_field() {
    let error = json("{\"port\": null}").parse::<Server>().unwrap_err();
    assert_eq!(one(&error), (Some(1), Some(10), "expected an integer"));
}

#[test]
fn a_float_too_large_is_one_problem_for_the_field() {
    let error = json("{\"ratio\": 1e400}")
        .parse::<BTreeMap<String, f64>>()
        .unwrap_err();
    assert_eq!(one(&error), (Some(1), Some(11), "expected a float"));
}

#[test]
fn an_escaped_duplicate_key_is_refused_on_the_second_key() {
    let text = "{\"name\": \"a\",\n\"\\u006eame\": \"b\"}";
    let error = json(text).parse::<BTreeMap<String, String>>().unwrap_err();
    assert_eq!(one(&error), (Some(2), Some(1), "duplicate key `name`"));
}

#[test]
fn a_listing_keeps_both_entries_of_a_duplicate_key() {
    let text = "{\"name\": \"a\",\n\"\\u006eame\": \"b\"}";
    let input = json(text);
    let entries = input.top_level().unwrap();
    let found: Vec<_> = entries
        .iter()
        .map(|entry| {
            let line = 1 + text[..entry.value.start].matches('\n').count();
            (
                entry.key.as_str(),
                line,
                entry.written,
                entry.string.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("name", 1, Written::DoubleQuoted, Some("a")),
            ("name", 2, Written::DoubleQuoted, Some("b")),
        ]
    );
}

#[test]
fn a_listing_tells_a_string_from_a_number_a_literal_and_a_collection() {
    let input = json("{\"a\": \"x\", \"b\": 1, \"c\": null, \"d\": [], \"e\": true}");
    let written: Vec<_> = input
        .top_level()
        .unwrap()
        .iter()
        .map(|entry| (entry.written, entry.string.clone()))
        .collect();
    assert_eq!(
        written,
        [
            (Written::DoubleQuoted, Some("x".to_owned())),
            (Written::Plain, None),
            (Written::Plain, None),
            (Written::Collection, None),
            (Written::Plain, None),
        ]
    );
}

#[test]
fn a_field_is_placed_on_its_value_and_its_key() {
    let input = json("{\n  \"server\": {\n    \"port\": 1\n  }\n}");
    let path = FieldPath::root().key("server").key("port");
    let value = input.place(&path).map(|at| (at.line, at.column));
    let key = input.place_key(&path).map(|at| (at.line, at.column));
    assert_eq!((value, key), (Some((3, 13)), Some((3, 5))));
}

#[test]
fn a_missing_field_sits_at_the_start_of_the_text() {
    let error = json("{}").parse::<Server>().unwrap_err();
    assert_eq!(one(&error), (Some(1), Some(1), "missing field `port`"));
}

#[test]
fn the_limits_name_their_field() {
    let nine = format!("{}{}", "[".repeat(9), "]".repeat(9));
    let limits = Limits {
        max_depth: Some(8),
        ..Limits::NONE
    };
    let error = json(&nine)
        .limits(limits)
        .parse::<IgnoredAny>()
        .unwrap_err();
    let (line, column, message) = one(&error);
    assert_eq!((line, column), (Some(1), Some(9)));
    assert!(message.starts_with("exceeds max_depth (8)"), "{message}");
}

#[test]
fn a_wide_text_reads_under_a_raised_limit() {
    let nine = format!("{}{}", "[".repeat(9), "]".repeat(9));
    let limits = Limits {
        max_depth: Some(9),
        ..Limits::NONE
    };
    assert!(json(&nine).limits(limits).parse::<IgnoredAny>().is_ok());
}

#[test]
fn the_same_field_reads_the_same_through_yaml_toml_and_json() {
    let formats: [(&'static dyn Format, &str, &str, &str); 3] = [
        (&YAML, "app.yaml", "port: 8080\n", "port: abc\n"),
        (&TOML, "app.toml", "port = 8080\n", "port = \"abc\"\n"),
        (
            &JSON,
            "app.json",
            "{\"port\": 8080}\n",
            "{\"port\": \"abc\"}\n",
        ),
    ];
    for (format, name, good, bad) in formats {
        let read = Input::new_with_format(name, good, format).parse::<Server>();
        assert_eq!(read.unwrap(), Server { port: 8080 }, "{name}");
        let error = Input::new_with_format(name, bad, format)
            .parse::<Server>()
            .unwrap_err();
        assert_eq!(one(&error).2, "expected an integer", "{name}");
    }
}

#[test]
fn a_formatted_file_reads_like_the_compact_one() {
    let text = indoc! {r#"
        {
          "port": 8080
        }
    "#};
    assert_eq!(json(text).parse::<Server>().unwrap(), Server { port: 8080 });
}

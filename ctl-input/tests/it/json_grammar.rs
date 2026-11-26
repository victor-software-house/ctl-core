//! The strict RFC 8259 grammar: each rule has a text that breaks it, the words
//! of the problem, and the bytes it sits at.

use ctl_input::Limits;
use ctl_input::format::{Fault, Format, JSON, Reading, Refusal};

use crate::common::{items, value};

fn refuse(text: &str) -> Refusal {
    match JSON.read(text, &Limits::NONE, Reading::Strict) {
        Ok(tree) => panic!("read {text:?} as {:?}", tree.kind()),
        Err(refusal) => refusal,
    }
}

/// The words of the syntax error `text` gets, and the bytes it sits at.
fn syntax(text: &str) -> (String, String) {
    let refusal = refuse(text);
    let Fault::Syntax { detail } = refusal.fault() else {
        panic!("{text:?} gave {:?}", refusal.fault());
    };
    let span = refusal
        .span()
        .unwrap_or_else(|| panic!("{refusal} has no span"));
    (detail.clone(), text[span].to_owned())
}

fn assert_refused(cases: &[(&str, &str, &str)]) {
    for (text, detail, at) in cases {
        assert_eq!(
            syntax(text),
            ((*detail).to_owned(), (*at).to_owned()),
            "{text:?}"
        );
    }
}

#[test]
fn a_value_is_expected_where_none_was_written() {
    assert_refused(&[
        ("", "expected a value", ""),
        ("  \n ", "expected a value", ""),
        ("[", "expected a value", ""),
        ("[,1]", "expected a value", ","),
        ("[1,]", "expected a value", "]"),
        ("[}", "expected a value", "}"),
        ("[:]", "expected a value", ":"),
        ("{\"a\":}", "expected a value", "}"),
        ("{\"a\":,}", "expected a value", ","),
        ("{\"a\"::1}", "expected a value", ":"),
        ("{\"a\": 1, \"b\": ]}", "expected a value", "]"),
    ]);
}

#[test]
fn a_container_that_is_not_closed_says_how_it_could_go_on() {
    assert_refused(&[
        ("[1", "expected `,` or `]`", ""),
        ("[1 2]", "expected `,` or `]`", "2"),
        ("[1}", "expected `,` or `]`", "}"),
        ("{\"a\": 1", "expected `,` or `}`", ""),
        ("{\"a\": 1 \"b\": 2}", "expected `,` or `}`", "\"b\""),
        ("{\"a\": 1]", "expected `,` or `}`", "]"),
    ]);
}

#[test]
fn a_key_is_a_string_in_double_quotes() {
    assert_refused(&[
        ("{", "expected a key in double quotes", ""),
        ("{1: 2}", "expected a key in double quotes", "1"),
        ("{a: 1}", "expected a key in double quotes", "a"),
        ("{'a': 1}", "expected a key in double quotes", "'"),
        ("{true: 1}", "expected a key in double quotes", "true"),
        ("{\"a\": 1,}", "expected a key in double quotes", "}"),
        ("{\"a\": 1, 2: 3}", "expected a key in double quotes", "2"),
        ("{,}", "expected a key in double quotes", ","),
    ]);
}

#[test]
fn a_key_is_followed_by_a_colon() {
    assert_refused(&[
        ("{\"a\"", "expected `:`", ""),
        ("{\"a\" 1}", "expected `:`", "1"),
        ("{\"a\", \"b\": 1}", "expected `:`", ","),
    ]);
}

#[test]
fn a_comment_a_trailing_comma_and_single_quotes_are_refused() {
    assert_refused(&[
        ("{\"a\": 1 // x\n}", "expected `,` or `}`", "/"),
        ("[1 /* x */]", "expected `,` or `]`", "/"),
        ("// x\n[]", "expected a value", "/"),
        ("['a']", "expected a value", "'"),
    ]);
}

#[test]
fn the_text_holds_one_value() {
    assert_refused(&[
        ("{} {}", "text after the value", "{"),
        ("[1] 2", "text after the value", "2"),
        ("1 2", "text after the value", "2"),
        ("\"a\"\"b\"", "text after the value", "\""),
        ("null,", "text after the value", ","),
        ("[]]", "text after the value", "]"),
        ("{}\n\n x", "text after the value", "x"),
    ]);
}

#[test]
fn only_four_characters_are_whitespace() {
    for (character, name) in [
        ('\u{b}', "vertical tab"),
        ('\u{c}', "form feed"),
        ('\u{a0}', "no-break space"),
        ('\u{2060}', "word joiner"),
        ('\u{2003}', "em space"),
        ('\u{85}', "next line"),
    ] {
        let text = format!("[{character}]");
        assert_eq!(
            syntax(&text),
            ("expected a value".to_owned(), character.to_string()),
            "{name}"
        );
    }
}

#[test]
fn a_number_follows_the_rfc_grammar() {
    assert_refused(&[
        ("[01]", "number `01` is not valid JSON", "01"),
        ("[-01]", "number `-01` is not valid JSON", "-01"),
        ("[00]", "number `00` is not valid JSON", "00"),
        ("[-]", "number `-` is not valid JSON", "-"),
        ("[1.]", "number `1.` is not valid JSON", "1."),
        ("[2.e3]", "number `2.e3` is not valid JSON", "2.e3"),
        ("[1e]", "number `1e` is not valid JSON", "1e"),
        ("[1e+]", "number `1e+` is not valid JSON", "1e+"),
        ("[1.5.2]", "number `1.5.2` is not valid JSON", "1.5.2"),
        ("[1-2]", "number `1-2` is not valid JSON", "1-2"),
        ("[-Infinity]", "number `-` is not valid JSON", "-"),
        (
            "{\"port\": 08080}",
            "number `08080` is not valid JSON",
            "08080",
        ),
        (".5", "expected a value", "."),
        ("[+1]", "expected a value", "+"),
        ("[Infinity]", "expected a value", "Infinity"),
        ("[NaN]", "expected a value", "NaN"),
        ("[0x1F]", "expected `,` or `]`", "x1F"),
    ]);
}

#[test]
fn a_literal_is_lower_case_and_ends_at_a_delimiter() {
    assert_refused(&[
        ("[True]", "expected a value", "True"),
        ("[NULL]", "expected a value", "NULL"),
        ("[nul]", "expected a value", "nul"),
        ("[truee]", "expected a value", "truee"),
        ("[nullx]", "expected a value", "nullx"),
        ("[true1]", "expected a value", "true1"),
        ("[truefalse]", "expected a value", "truefalse"),
        ("[undefined]", "expected a value", "undefined"),
    ]);
}

#[test]
fn a_string_is_closed_and_holds_no_raw_control_character() {
    assert_refused(&[
        ("\"abc", "a string is not closed", "\"abc"),
        ("[\"abc", "a string is not closed", "\"abc"),
        ("\"abc\\", "a string is not closed", "\"abc\\"),
        (
            "[\"a\tb\"]",
            "control character U+0009 must be escaped",
            "\t",
        ),
        (
            "[\"a\nb\"]",
            "control character U+000A must be escaped",
            "\n",
        ),
        (
            "[\"a\rb\"]",
            "control character U+000D must be escaped",
            "\r",
        ),
        (
            "[\"a\u{0}b\"]",
            "control character U+0000 must be escaped",
            "\u{0}",
        ),
        (
            "[\"a\u{1f}b\"]",
            "control character U+001F must be escaped",
            "\u{1f}",
        ),
        (
            "{\"a\u{1}\": 1}",
            "control character U+0001 must be escaped",
            "\u{1}",
        ),
    ]);
}

#[test]
fn a_string_takes_only_the_escapes_the_rfc_lists() {
    assert_refused(&[
        ("[\"\\q\"]", "unknown escape `\\q`", "\\q"),
        ("[\"\\'\"]", "unknown escape `\\'`", "\\'"),
        ("[\"\\x41\"]", "unknown escape `\\x`", "\\x"),
        ("[\"\\é\"]", "unknown escape `\\é`", "\\é"),
        ("[\"\\u12g4\"]", "unknown escape `\\u12g`", "\\u12g"),
        ("[\"\\u\"]", "unknown escape `\\u\"`", "\\u\""),
        ("[\"\\u12\"]", "unknown escape `\\u12\"`", "\\u12\""),
    ]);
}

#[test]
fn a_lone_or_inverted_surrogate_escape_is_refused() {
    let half = |escape: &str| format!("escape `{escape}` is half of a surrogate pair");
    assert_refused(&[
        ("[\"\\ud800\"]", &half("\\ud800"), "\\ud800"),
        ("[\"\\uDBFF\"]", &half("\\uDBFF"), "\\uDBFF"),
        ("[\"\\udc00\"]", &half("\\udc00"), "\\udc00"),
        ("[\"\\udfff\\ud800\"]", &half("\\udfff"), "\\udfff"),
        ("[\"\\ud800\\u0041\"]", &half("\\ud800"), "\\ud800"),
        ("[\"\\ud800x\"]", &half("\\ud800"), "\\ud800"),
        ("[\"\\ud800\\ud800\"]", &half("\\ud800"), "\\ud800"),
        ("{\"\\udc00\": 1}", &half("\\udc00"), "\\udc00"),
    ]);
}

#[test]
fn a_surrogate_pair_and_a_nul_escape_are_decoded() {
    let text = "[\"\\ud83d\\ude00\", \"\\u0000\"]";
    let root = JSON
        .read(text, &Limits::NONE, Reading::Strict)
        .unwrap_or_else(|refusal| panic!("{refusal}"));
    let values: Vec<_> = items(&root).iter().map(value).collect();
    assert_eq!(values, ["\u{1f600}", "\u{0}"]);
}

#[test]
fn a_syntax_error_has_no_parser_name_in_its_words() {
    for text in ["[1,]", "{\"a\" 1}", "[01]", "[\"\\q\"]", "{} 1", ""] {
        let words = refuse(text).to_string();
        for parser in ["serde", "json_scanner", "expected value at", "EOF while"] {
            assert!(!words.contains(parser), "{words}");
        }
    }
}

#[test]
fn nesting_past_max_depth_is_refused_at_the_array_that_opens_it() {
    let limits = Limits {
        max_depth: Some(8),
        ..Limits::NONE
    };
    let nine = format!("{}{}", "[".repeat(9), "]".repeat(9));
    let refusal = JSON.read(&nine, &limits, Reading::Strict).unwrap_err();
    assert_eq!(refusal.span(), Some(8..9));
    assert!(
        matches!(refusal.fault(), Fault::Limit { name: "max_depth", value, .. } if value == "8"),
        "{:?}",
        refusal.fault()
    );
    let eight = format!("{}{}", "[".repeat(8), "]".repeat(8));
    assert!(JSON.read(&eight, &limits, Reading::Strict).is_ok());
}

#[test]
fn the_root_container_is_the_first_level_and_an_object_counts_as_one() {
    let limits = Limits {
        max_depth: Some(2),
        ..Limits::NONE
    };
    assert!(JSON.read("{\"a\": []}", &limits, Reading::Strict).is_ok());
    let refusal = JSON
        .read("{\"a\": [{}]}", &limits, Reading::Strict)
        .unwrap_err();
    assert_eq!(refusal.span(), Some(7..8));
    assert!(matches!(
        refusal.fault(),
        Fault::Limit {
            name: "max_depth",
            ..
        }
    ));
}

#[test]
fn a_text_past_max_nodes_is_refused_at_the_node_that_goes_past_it() {
    let limits = Limits {
        max_nodes: Some(3),
        ..Limits::NONE
    };
    assert!(JSON.read("[1, 2]", &limits, Reading::Strict).is_ok());
    let refusal = JSON
        .read("[1, 2, 3]", &limits, Reading::Strict)
        .unwrap_err();
    assert_eq!(refusal.span(), Some(7..8));
    assert!(
        matches!(refusal.fault(), Fault::Limit { name: "max_nodes", value, .. } if value == "3"),
        "{:?}",
        refusal.fault()
    );
    let two = Limits {
        max_nodes: Some(2),
        ..Limits::NONE
    };
    let objects = JSON.read("{\"a\": [], \"b\": {}}", &two, Reading::Strict);
    assert_eq!(objects.unwrap_err().span(), Some(15..16));
}

#[test]
fn a_text_past_max_reader_input_bytes_is_refused_before_the_first_token() {
    let limits = Limits {
        max_reader_input_bytes: Some(4),
        ..Limits::NONE
    };
    assert!(JSON.read("[1,2]", &Limits::NONE, Reading::Strict).is_ok());
    let refusal = JSON.read("[1,2]", &limits, Reading::Strict).unwrap_err();
    assert_eq!(refusal.span(), None);
    assert!(matches!(
        refusal.fault(),
        Fault::Limit {
            name: "max_reader_input_bytes",
            ..
        }
    ));
    // The text breaks the grammar too, and the size is what the read names.
    let broken = JSON.read("[,,,,]", &limits, Reading::Strict).unwrap_err();
    assert!(matches!(broken.fault(), Fault::Limit { .. }));
}

#[test]
fn a_limit_the_text_hits_before_a_syntax_error_wins() {
    let limits = Limits {
        max_depth: Some(1),
        ..Limits::NONE
    };
    let refusal = JSON.read("[[,", &limits, Reading::Strict).unwrap_err();
    assert!(matches!(
        refusal.fault(),
        Fault::Limit {
            name: "max_depth",
            ..
        }
    ));
}

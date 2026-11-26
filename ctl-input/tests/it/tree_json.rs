//! The tree a JSON text builds: each value and key keeps its bytes, an object
//! its entries in the order written, a number its text and a literal its
//! syntax.

use std::mem;

use ctl_input::format::{Fault, JSON, Reading};
use ctl_input::tree::{Node, NodeKind, ScalarStyle, ScalarSyntax};
use ctl_input::{Format, Limits};
use indoc::indoc;

use crate::common::{entries, get, items, read, source, value};

fn syntax_of(node: &Node) -> Option<ScalarSyntax> {
    let NodeKind::Scalar(scalar) = node.kind() else {
        panic!("not a scalar");
    };
    scalar.syntax()
}

fn style_of(node: &Node) -> ScalarStyle {
    let NodeKind::Scalar(scalar) = node.kind() else {
        panic!("not a scalar");
    };
    scalar.style()
}

#[test]
fn an_object_keeps_its_entries_in_the_order_written() {
    let text = "{\"b\": 1, \"a\": 2, \"c\": 3}";
    let root = read(&JSON, text);
    let keys: Vec<_> = entries(&root).iter().map(|(key, _)| value(key)).collect();
    assert_eq!(keys, ["b", "a", "c"]);
    assert_eq!(source(text, &root), text);
}

#[test]
fn a_string_spans_its_quotes_and_a_key_is_the_same_kind_of_scalar() {
    let text = "{\"name\": \"release\"}";
    let root = read(&JSON, text);
    let (key, found) = &entries(&root)[0];
    assert_eq!(source(text, key), "\"name\"");
    assert_eq!(source(text, found), "\"release\"");
    for node in [key, found] {
        assert_eq!(style_of(node), ScalarStyle::DoubleQuoted);
        assert_eq!(syntax_of(node), Some(ScalarSyntax::Text));
    }
}

#[test]
fn an_array_is_a_sequence_and_an_empty_one_has_no_items() {
    let text = "{\"a\": [1, [], {}], \"b\": []}";
    let root = read(&JSON, text);
    let list = items(get(&root, "a"));
    assert_eq!(list.len(), 3);
    assert_eq!(source(text, &list[1]), "[]");
    assert_eq!(entries(&list[2]).len(), 0);
    assert_eq!(items(get(&root, "b")).len(), 0);
}

#[test]
fn a_number_keeps_its_text_and_is_an_integer_unless_it_has_a_fraction_or_exponent() {
    let text = "[0, -0, 18446744073709551616, 1.5, -2.50, 1e3, 1E-3, 0.0e+0]";
    let root = read(&JSON, text);
    let found: Vec<_> = items(&root)
        .iter()
        .map(|node| (value(node), syntax_of(node), style_of(node)))
        .collect();
    let integer = Some(ScalarSyntax::Integer);
    let float = Some(ScalarSyntax::Float);
    assert_eq!(
        found,
        [
            ("0", integer, ScalarStyle::Plain),
            ("-0", integer, ScalarStyle::Plain),
            ("18446744073709551616", integer, ScalarStyle::Plain),
            ("1.5", float, ScalarStyle::Plain),
            ("-2.50", float, ScalarStyle::Plain),
            ("1e3", float, ScalarStyle::Plain),
            ("1E-3", float, ScalarStyle::Plain),
            ("0.0e+0", float, ScalarStyle::Plain),
        ]
    );
}

#[test]
fn the_literals_have_their_own_syntax() {
    let root = read(&JSON, "[true, false, null]");
    let found: Vec<_> = items(&root)
        .iter()
        .map(|node| (value(node), syntax_of(node)))
        .collect();
    assert_eq!(
        found,
        [
            ("true", Some(ScalarSyntax::Boolean)),
            ("false", Some(ScalarSyntax::Boolean)),
            ("null", Some(ScalarSyntax::Null)),
        ]
    );
}

#[test]
fn a_node_carries_no_tag() {
    let root = read(&JSON, "[1]");
    assert!(root.tag().is_none());
    assert!(items(&root)[0].tag().is_none());
}

#[test]
fn a_root_scalar_is_a_document() {
    for (text, expected) in [
        ("\"a\"", "a"),
        (" 12 ", "12"),
        ("true", "true"),
        ("null", "null"),
    ] {
        assert_eq!(value(&read(&JSON, text)), expected, "{text}");
    }
}

#[test]
fn escapes_are_decoded_in_the_value_and_kept_in_the_span() {
    let text = r#"["a\nb", "\u00e9", "\ud834\udd1e", "\/\"\\\b\f\r\t", "é\u0041"]"#;
    let root = read(&JSON, text);
    let decoded: Vec<_> = items(&root).iter().map(value).collect();
    assert_eq!(
        decoded,
        ["a\nb", "é", "\u{1d11e}", "/\"\\\u{8}\u{c}\r\t", "éA"]
    );
    assert_eq!(source(text, &items(&root)[1]), r#""\u00e9""#);
}

#[test]
fn a_raw_del_and_a_line_separator_are_accepted_in_a_string() {
    let root = read(&JSON, "[\"\u{7f}\u{2028}\u{2029}\"]");
    assert_eq!(value(&items(&root)[0]), "\u{7f}\u{2028}\u{2029}");
}

#[test]
fn whitespace_is_space_tab_line_feed_and_carriage_return() {
    let root = read(&JSON, " \t\r\n[ \t\r\n1 \t\r\n, \t\r\n2 \t\r\n] \t\r\n");
    assert_eq!(items(&root).len(), 2);
}

#[test]
fn a_leading_byte_order_mark_is_skipped_and_spans_stay_in_the_text() {
    let text = "\u{feff}{\"a\": 1}";
    let root = read(&JSON, text);
    assert_eq!(source(text, &root), "{\"a\": 1}");
    assert_eq!(source(text, get(&root, "a")), "1");
}

#[test]
fn a_strict_reading_refuses_a_key_written_twice_on_the_second() {
    let text = "{\"name\": \"a\",\n\"\\u006eame\": \"b\"}";
    let refusal = JSON.read(text, &Limits::NONE, Reading::Strict).unwrap_err();
    assert_eq!(
        refusal.fault(),
        &Fault::DuplicateKey {
            key: "name".to_owned()
        }
    );
    assert_eq!(&text[refusal.span().unwrap()], "\"\\u006eame\"");
}

#[test]
fn a_duplicate_key_in_another_object_is_no_duplicate() {
    let root = read(&JSON, "{\"a\": {\"a\": 1}, \"b\": {\"a\": 2}}");
    assert_eq!(entries(&root).len(), 2);
}

#[test]
fn a_listing_reading_keeps_both_entries() {
    let text = "{\"name\": \"a\",\n\"\\u006eame\": \"b\"}";
    let root = JSON
        .read(text, &Limits::NONE, Reading::Listing)
        .unwrap_or_else(|refusal| panic!("{refusal}"));
    let found: Vec<_> = entries(&root)
        .iter()
        .map(|(key, found)| (value(key), value(found)))
        .collect();
    assert_eq!(found, [("name", "a"), ("name", "b")]);
}

#[test]
fn a_deep_text_reads_without_recursing() {
    let depth = 5000;
    let text = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    let limits = Limits {
        max_depth: Some(depth),
        ..Limits::NONE
    };
    let root = JSON
        .read(&text, &limits, Reading::Strict)
        .unwrap_or_else(|refusal| panic!("{refusal}"));
    assert_eq!(root.span(), 0..text.len());
    // Dropping a tree this deep recurses, so it is leaked after the check.
    mem::forget(root);
}

#[test]
fn a_nested_text_keeps_every_span() {
    let text = indoc! {r#"
        {
          "a": [1, {"b": null}],
          "c": "d"
        }
    "#};
    let root = read(&JSON, text);
    let list = items(get(&root, "a"));
    assert_eq!(source(text, &list[0]), "1");
    assert_eq!(source(text, get(&list[1], "b")), "null");
    assert_eq!(source(text, get(&root, "c")), "\"d\"");
    assert_eq!(source(text, &root), text.trim_end());
}

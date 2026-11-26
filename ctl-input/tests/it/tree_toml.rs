//! The TOML tree: spans, styles, canonical scalar text and source order.

use ctl_input::format::TOML;
use ctl_input::tree::{Node, NodeKind, ScalarStyle};
use indoc::indoc;

use crate::common::{entries, get, items, read, source, value};

fn style(node: &Node) -> ScalarStyle {
    let NodeKind::Scalar(scalar) = node.kind() else {
        panic!("not a scalar");
    };
    scalar.style()
}

#[test]
fn the_four_kinds_of_string_keep_their_source_and_style() {
    let text = indoc! {r#"
        basic = "a\tb"
        literal = 'c:\d'
        multi = """
        two
        lines"""
        raw = '''
        raw lines'''
    "#};
    let root = read(&TOML, text);
    let expected = [
        ("basic", "a\tb", ScalarStyle::DoubleQuoted, r#""a\tb""#),
        ("literal", "c:\\d", ScalarStyle::SingleQuoted, r"'c:\d'"),
        (
            "multi",
            "two\nlines",
            ScalarStyle::MultilineBasic,
            "\"\"\"\ntwo\nlines\"\"\"",
        ),
        (
            "raw",
            "raw lines",
            ScalarStyle::MultilineLiteral,
            "'''\nraw lines'''",
        ),
    ];
    for (key, processed, written, spelled) in expected {
        let node = get(&root, key);
        assert_eq!(value(node), processed, "{key}");
        assert_eq!(style(node), written, "{key}");
        assert_eq!(source(text, node), spelled, "{key}");
    }
}

#[test]
fn numbers_are_canonical_decimal_text_and_plain() {
    let text = indoc! {"
        hex = 0xFF
        octal = 0o17
        binary = 0b101
        decimal = 1_000
        negative = -5
        float = 3.14
        exponent = 1e3
        infinite = inf
        yes = true
        when = 1979-05-27T07:32:00Z
    "};
    let root = read(&TOML, text);
    let expected = [
        ("hex", "255", "0xFF"),
        ("octal", "15", "0o17"),
        ("binary", "5", "0b101"),
        ("decimal", "1000", "1_000"),
        ("negative", "-5", "-5"),
        ("float", "3.14", "3.14"),
        ("exponent", "1e3", "1e3"),
        ("infinite", "inf", "inf"),
        ("yes", "true", "true"),
        ("when", "1979-05-27T07:32:00Z", "1979-05-27T07:32:00Z"),
    ];
    for (key, processed, spelled) in expected {
        let node = get(&root, key);
        assert_eq!(value(node), processed, "{key}");
        assert_eq!(style(node), ScalarStyle::Plain, "{key}");
        assert_eq!(source(text, node), spelled, "{key}");
    }
}

#[test]
fn entries_keep_the_order_written_and_keys_keep_their_style() {
    let text = "zebra = 1\n\"quoted.key\" = 2\n'literal key' = 3\napple = 4\n";
    let root = read(&TOML, text);
    let keys: Vec<_> = entries(&root)
        .iter()
        .map(|(key, _)| (value(key), style(key), source(text, key)))
        .collect();
    assert_eq!(
        keys,
        [
            ("zebra", ScalarStyle::Plain, "zebra"),
            ("quoted.key", ScalarStyle::DoubleQuoted, "\"quoted.key\""),
            ("literal key", ScalarStyle::SingleQuoted, "'literal key'"),
            ("apple", ScalarStyle::Plain, "apple"),
        ]
    );
}

#[test]
fn tables_arrays_of_tables_inline_tables_and_dotted_keys_nest() {
    let text = indoc! {"
        a.b.c = 1
        point = { x = 1, y = 2 }
        tags = [\"x\", \"y\"]

        [server]
        port = 8080

        [[products]]
        name = \"a\"

        [[products]]
        name = \"b\"
    "};
    let root = read(&TOML, text);
    assert_eq!(source(text, get(get(get(&root, "a"), "b"), "c")), "1");
    let point = get(&root, "point");
    assert_eq!(source(text, point), "{ x = 1, y = 2 }");
    assert_eq!(source(text, get(point, "y")), "2");
    let tags: Vec<_> = items(get(&root, "tags")).iter().map(value).collect();
    assert_eq!(tags, ["x", "y"]);
    assert_eq!(source(text, get(get(&root, "server"), "port")), "8080");
    let products = items(get(&root, "products"));
    let names: Vec<_> = products
        .iter()
        .map(|table| value(get(table, "name")))
        .collect();
    assert_eq!(names, ["a", "b"]);
    assert!(root.used_at().is_none() && root.tag().is_none());
}

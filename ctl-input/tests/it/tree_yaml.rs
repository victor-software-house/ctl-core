//! The YAML tree: spans, styles, tags and the alias that replayed a node.

use ctl_input::Limits;
use ctl_input::format::{Format, Reading, YAML};
use ctl_input::tree::{Node, NodeKind, ScalarStyle, Tag};
use indoc::indoc;

use crate::common::{entries, get, items, read, source, value};

fn style(node: &Node) -> ScalarStyle {
    let NodeKind::Scalar(scalar) = node.kind() else {
        panic!("not a scalar");
    };
    scalar.style()
}

#[test]
fn scalars_keep_the_bytes_and_the_style_they_were_written_in() {
    let text = indoc! {"
        plain: demo
        double: \"a\\tb\"
        single: 'it''s'
        literal: |
          one
          two
        folded: >-
          fold
          ed
    "};
    let root = read(&YAML, text);
    let expected = [
        ("plain", "demo", ScalarStyle::Plain, "demo"),
        ("double", "a\tb", ScalarStyle::DoubleQuoted, "\"a\\tb\""),
        ("single", "it's", ScalarStyle::SingleQuoted, "'it''s'"),
        (
            "literal",
            "one\ntwo\n",
            ScalarStyle::Literal,
            "|\n  one\n  two",
        ),
        ("folded", "fold ed", ScalarStyle::Folded, ">-\n  fold\n  ed"),
    ];
    for (key, processed, written, spelled) in expected {
        let node = get(&root, key);
        assert_eq!(value(node), processed, "{key}");
        assert_eq!(style(node), written, "{key}");
        assert_eq!(source(text, node), spelled, "{key}");
    }
}

#[test]
fn keys_are_scalar_nodes_with_their_own_spans() {
    let text = "'a.b': 1\n\"c d\": 2\nplain: 3\n";
    let root = read(&YAML, text);
    let keys: Vec<_> = entries(&root)
        .iter()
        .map(|(key, _)| (value(key), style(key), source(text, key)))
        .collect();
    assert_eq!(
        keys,
        [
            ("a.b", ScalarStyle::SingleQuoted, "'a.b'"),
            ("c d", ScalarStyle::DoubleQuoted, "\"c d\""),
            ("plain", ScalarStyle::Plain, "plain"),
        ]
    );
}

#[test]
fn flow_and_block_collections_keep_their_items_in_order() {
    let text = indoc! {"
        flow: [1, {k: v}]
        block:
          inner:
            - x
            - y
    "};
    let root = read(&YAML, text);
    let flow = get(&root, "flow");
    assert_eq!(source(text, flow), "[1, {k: v}]");
    let [one, map] = items(flow) else {
        panic!("two items");
    };
    assert_eq!(source(text, one), "1");
    assert_eq!(source(text, map), "{k: v}");
    assert_eq!(source(text, get(map, "k")), "v");
    let list = get(get(&root, "block"), "inner");
    let spelled: Vec<_> = items(list).iter().map(|item| source(text, item)).collect();
    assert_eq!(spelled, ["x", "y"]);
    assert!(root.used_at().is_none());
}

#[test]
fn a_tag_is_kept_in_the_spelling_serde_saphyr_gives() {
    let text = "local: !override 0\nstring: !!str 12\nuntagged: 1\n";
    let root = read(&YAML, text);
    let tag = |key| get(&root, key).tag().map(Tag::as_str);
    assert_eq!(tag("local"), Some("!override"));
    assert_eq!(tag("string"), Some("tag:yaml.org,2002:str"));
    assert_eq!(tag("untagged"), None);
    assert_eq!(source(text, get(&root, "local")), "0");
}

#[test]
fn an_alias_replays_its_anchor_and_marks_where_it_was_used() {
    let text = "a: &x [1, 2]\nb: *x\n";
    let root = read(&YAML, text);
    let anchored = get(&root, "a");
    assert!(anchored.used_at().is_none());
    let used = get(&root, "b");
    assert_eq!(
        source(text, used),
        "[1, 2]",
        "a replay keeps the anchor's span"
    );
    assert_eq!(used.used_at().map(|at| &text[at]), Some("*x"));
    for item in items(used) {
        assert_eq!(item.used_at().map(|at| &text[at]), Some("*x"), "inherited");
    }
}

#[test]
fn the_outermost_alias_wins_over_a_nested_one() {
    let text = indoc! {"
        one: &one {k: v}
        two: &two [*one]
        three: *two
    "};
    let root = read(&YAML, text);
    let inside_two = &items(get(&root, "two"))[0];
    assert_eq!(inside_two.used_at().map(|at| &text[at]), Some("*one"));
    let three = get(&root, "three");
    assert_eq!(three.used_at().map(|at| &text[at]), Some("*two"));
    let nested = &items(three)[0];
    assert_eq!(
        nested.used_at().map(|at| &text[at]),
        Some("*two"),
        "the outermost alias is inherited, not the one written inside the anchor"
    );
    assert_eq!(source(text, nested), "{k: v}");
    assert_eq!(nested.used_at(), three.used_at());
    assert_eq!(get(nested, "k").used_at(), three.used_at());
}

#[test]
fn a_listing_reads_the_same_tree_as_a_strict_reading() {
    let text = "a: 1\nb: [x]\n";
    let strict = YAML.read(text, &Limits::NONE, Reading::Strict).unwrap();
    let listing = YAML.read(text, &Limits::NONE, Reading::Listing).unwrap();
    assert_eq!(strict, listing);
}

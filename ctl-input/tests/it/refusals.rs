//! What a read refuses, and where: strict and listing readings, the limits,
//! and aliases.

use std::iter;

use ctl_input::Limits;
use ctl_input::format::{Fault, Format, Raise, Reading, Refusal, TOML, YAML};
use ctl_input::tree::Node;
use serde::de::IgnoredAny;

use crate::common::{entries, source};

fn strict(format: &dyn Format, text: &str, limits: &Limits) -> Result<Node, Refusal> {
    format.read(text, limits, Reading::Strict)
}

/// Nine anchors, each a list of ten aliases of the one before.
fn alias_bomb() -> String {
    let first = format!("l0: &l0 [{}]\n", ["x"; 10].join(", "));
    let rest = (1..9).map(|level| {
        let aliases = vec![format!("*l{}", level - 1); 10].join(", ");
        format!("l{level}: &l{level} [{aliases}]\n")
    });
    iter::once(first).chain(rest).collect()
}

#[test]
fn a_strict_reading_refuses_a_duplicate_key_at_the_second_one() {
    let text = "a: 1\nb: 2\na: 3\n";
    let refusal = strict(&YAML, text, &Limits::NONE).unwrap_err();
    assert_eq!(
        refusal.fault(),
        &Fault::DuplicateKey {
            key: "a".to_owned()
        }
    );
    assert_eq!(refusal.span().map(|span| &text[span]), Some("a"));
    assert_eq!(refusal.span().map(|span| span.start), Some(10));
    assert_eq!(refusal.defined_at(), None);
}

#[test]
fn a_listing_keeps_a_duplicate_key_as_an_entry() {
    let text = "a: 1\nb: 2\na: 3\n";
    let root = YAML.read(text, &Limits::NONE, Reading::Listing).unwrap();
    let keys: Vec<_> = entries(&root)
        .iter()
        .map(|(key, _)| source(text, key))
        .collect();
    assert_eq!(keys, ["a", "b", "a"]);
}

#[test]
fn a_strict_reading_refuses_a_merge_key() {
    let text = "base: &b {x: 1}\n<<: *b\n";
    let refusal = strict(&YAML, text, &Limits::NONE).unwrap_err();
    assert_eq!(refusal.fault(), &Fault::MergeKey);
    let span = refusal.span().unwrap();
    assert_eq!(&text[span], "<<");
}

#[test]
fn a_listing_keeps_a_merge_key_as_an_ordinary_entry() {
    let text = "base: &b {x: 1}\n<<: *b\n";
    let root = YAML.read(text, &Limits::NONE, Reading::Listing).unwrap();
    let keys: Vec<_> = entries(&root)
        .iter()
        .map(|(key, _)| source(text, key))
        .collect();
    assert_eq!(keys, ["base", "<<"]);
}

#[test]
fn a_syntax_error_is_a_syntax_fault_with_a_span() {
    let texts = [
        "a: [\n",
        "a:\n\t- b\n",
        "a: *x\n",
        "🔑: [é, *x\n",
        "a: 1\n---\nb: 2\n",
    ];
    for text in texts {
        let refusal = strict(&YAML, text, &Limits::NONE).unwrap_err();
        assert!(matches!(refusal.fault(), Fault::Syntax { .. }), "{text:?}");
        let span = refusal
            .span()
            .unwrap_or_else(|| panic!("no span for {text:?}"));
        assert_eq!(span.start, span.end, "{text:?}");
        let before = &text[..span.start];
        let line = u64::try_from(before.matches('\n').count() + 1).unwrap();
        let last = before.rsplit('\n').next().unwrap_or_default();
        let column = u64::try_from(last.chars().count() + 1).unwrap();
        let error = serde_saphyr::from_str::<IgnoredAny>(text).unwrap_err();
        let at = error.without_snippet().location().unwrap();
        assert_eq!((line, column), (at.line(), at.column()), "{text:?}");
    }
    let second = "a: 1\n---\nb: 2\n";
    let refusal = strict(&YAML, second, &Limits::NONE).unwrap_err();
    assert_eq!(refusal.span(), Some(9..9));
    let refusal = strict(&TOML, "a = \n", &Limits::NONE).unwrap_err();
    assert!(matches!(refusal.fault(), Fault::Syntax { .. }));
    assert!(refusal.span().is_some());
}

#[test]
fn toml_has_no_duplicate_or_merge_keys_so_both_readings_agree() {
    let text = "a = 1\n[b]\nc = 2\n";
    let strict = strict(&TOML, text, &Limits::NONE).unwrap();
    let listing = TOML.read(text, &Limits::NONE, Reading::Listing).unwrap();
    assert_eq!(strict, listing);
    let refusal = TOML
        .read("a = 1\na = 2\n", &Limits::NONE, Reading::Listing)
        .unwrap_err();
    assert!(matches!(refusal.fault(), Fault::Syntax { .. }));
}

fn limit(name: &'static str, value: &str) -> Fault {
    Fault::Limit {
        name,
        value: value.to_owned(),
        raise: Raise::InRepository,
    }
}

#[test]
fn each_builtin_reports_a_limit_breach_naming_the_field() {
    let depth = Limits {
        max_depth: Some(2),
        ..Limits::NONE
    };
    let nodes = Limits {
        max_nodes: Some(3),
        ..Limits::NONE
    };
    let bytes = Limits {
        max_reader_input_bytes: Some(8),
        ..Limits::NONE
    };
    let yaml = "a: {b: {c: {d: 1}}}\n";
    let toml = "[a.b.c]\nd = 1\n";
    let texts: [(&dyn Format, &str); 2] = [(&YAML, yaml), (&TOML, toml)];
    for (format, text) in texts {
        let refusal = strict(format, text, &depth).unwrap_err();
        assert_eq!(refusal.fault(), &limit("max_depth", "2"), "{text}");
        assert!(refusal.span().is_some(), "{text}");
        let refusal = strict(format, text, &nodes).unwrap_err();
        assert_eq!(refusal.fault(), &limit("max_nodes", "3"), "{text}");
        let refusal = strict(format, text, &bytes).unwrap_err();
        assert_eq!(
            refusal.fault(),
            &limit("max_reader_input_bytes", "8"),
            "{text}"
        );
    }
}

#[test]
fn a_yaml_only_limit_is_a_fault_naming_the_budget_field() {
    let limits = Limits {
        max_aliases: Some(2),
        ..Limits::NONE
    };
    let refusal = strict(&YAML, "a: &x 1\nb: [*x, *x, *x]\n", &limits).unwrap_err();
    assert_eq!(refusal.fault(), &limit("max_aliases", "2"));
}

#[test]
fn nested_aliases_past_the_replay_limit_refuse_before_the_tree_is_built() {
    let limits = Limits {
        max_total_replayed_events: Some(1000),
        ..Limits::NONE
    };
    let text = alias_bomb();
    let refusal = strict(&YAML, &text, &limits).unwrap_err();
    assert_eq!(refusal.fault(), &limit("max_total_replayed_events", "1000"));
    let alias = refusal.span().map(|span| &text[span]);
    assert!(
        alias.is_some_and(|spelled| spelled.starts_with('*')),
        "{alias:?}"
    );
    let defined = refusal.defined_at().unwrap();
    assert!(
        defined.start < text.find("l1:").unwrap(),
        "defined in the first anchor"
    );
}

#[test]
fn the_per_anchor_replay_limit_names_its_own_field() {
    let text = "a: &a [x]\nb: [*a, *a]\n";
    let limits = Limits {
        max_alias_expansions_per_anchor: Some(1),
        ..Limits::NONE
    };
    let refusal = strict(&YAML, text, &limits).unwrap_err();
    assert_eq!(
        refusal.fault(),
        &limit("max_alias_expansions_per_anchor", "1")
    );
    assert!(strict(&YAML, text, &Limits::NONE).is_ok());
}

#[test]
fn an_alias_inside_its_own_anchor_is_refused() {
    let text = "a: &a [*a]\n";
    let refusal = strict(&YAML, text, &Limits::NONE).unwrap_err();
    assert!(
        matches!(refusal.fault(), Fault::Syntax { .. }),
        "{refusal:?}"
    );
}

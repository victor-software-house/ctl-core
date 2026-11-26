//! The tree an `Input` keeps: one read for every operation, read again only
//! when the limits or the kind of reading change.

use std::sync::atomic::{AtomicUsize, Ordering};

use ctl_input::format::{Mismatch, Reading, Refusal, Resolved, Wanted, YAML};
use ctl_input::tree::{Node, Scalar, Tag};
use ctl_input::{FieldPath, Format, Input, Limits};
use serde::de::IgnoredAny;

/// A format that reads as YAML does and counts its reads.
struct Counting(AtomicUsize);

impl Counting {
    const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    fn reads(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

impl Format for Counting {
    fn name(&self) -> &'static str {
        "counting"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["count"]
    }

    fn read(&self, text: &str, limits: &Limits, reading: Reading) -> Result<Node, Refusal> {
        self.0.fetch_add(1, Ordering::Relaxed);
        YAML.read(text, limits, reading)
    }

    fn resolve<'a>(
        &self,
        scalar: &'a Scalar,
        tag: Option<&Tag>,
        wanted: Wanted,
    ) -> Result<Resolved<'a>, Mismatch> {
        YAML.resolve(scalar, tag, wanted)
    }
}

const NESTED: &str = "a: {b: {c: {d: 1}}}\n";

fn tight() -> Limits {
    Limits {
        max_depth: Some(2),
        ..Limits::NONE
    }
}

fn first() -> FieldPath {
    FieldPath::root().key("a")
}

#[test]
fn a_second_placing_reuses_the_tree() {
    static FORMAT: Counting = Counting::new();
    let input = Input::new_with_format("a.count", "files:\n  'a.b':\n    label: ''\n", &FORMAT);
    let path = FieldPath::root().key("files").key("a.b").key("label");
    assert_eq!(FORMAT.reads(), 0);
    assert_eq!(input.place(&path).map(|at| at.line), Some(3));
    assert_eq!(input.place(&path).map(|at| at.line), Some(3));
    assert_eq!(input.place_key(&path).map(|at| at.line), Some(3));
    assert_eq!(FORMAT.reads(), 1);
}

#[test]
fn listing_the_top_level_reads_the_tree_that_placing_reuses() {
    static FORMAT: Counting = Counting::new();
    let input = Input::new_with_format("a.count", "a: 1\n", &FORMAT);
    input.top_level().unwrap();
    assert!(input.place(&first()).is_some());
    assert_eq!(FORMAT.reads(), 1);
}

#[test]
fn a_parse_reads_the_tree_that_placing_reuses() {
    static FORMAT: Counting = Counting::new();
    let input = Input::new_with_format("a.count", "a: 1\n", &FORMAT);
    input.parse::<IgnoredAny>().unwrap();
    assert!(input.place(&first()).is_some());
    assert_eq!(input.top_level().unwrap().len(), 1);
    assert_eq!(FORMAT.reads(), 1);
}

#[test]
fn a_parse_after_a_listing_reads_a_strict_tree() {
    static FORMAT: Counting = Counting::new();
    let input = Input::new_with_format("a.count", "prefix: a\nprefix: b\n", &FORMAT);
    assert_eq!(input.top_level().unwrap().len(), 2);
    let error = input.parse::<IgnoredAny>().unwrap_err();
    assert_eq!(error.to_string(), "a.count:2:1: duplicate key `prefix`");
    assert_eq!(FORMAT.reads(), 2);
}

#[test]
fn a_refused_strict_read_does_not_stop_placing() {
    static FORMAT: Counting = Counting::new();
    let input = Input::new_with_format("a.count", "a: 1\na: 2\n", &FORMAT);
    assert!(input.parse::<IgnoredAny>().is_err());
    assert!(input.parse::<IgnoredAny>().is_err());
    assert_eq!(FORMAT.reads(), 1);
    assert_eq!(input.place(&first()).map(|at| at.line), Some(1));
    assert_eq!(FORMAT.reads(), 2);
}

#[test]
fn a_text_over_the_size_limit_reads_no_tree() {
    static FORMAT: Counting = Counting::new();
    let limits = Limits {
        max_reader_input_bytes: Some(4),
        ..Limits::NONE
    };
    let input = Input::new_with_format("a.count", "a: 1\nb: 2\n", &FORMAT).limits(limits);
    assert_eq!(input.place(&first()), None);
    assert_eq!(input.place_key(&first()), None);
    assert!(input.parse::<IgnoredAny>().is_err());
    assert!(input.top_level().is_err());
    assert_eq!(FORMAT.reads(), 0);
}

#[test]
fn a_text_over_the_depth_or_node_limit_is_placed_nowhere() {
    let at = |limits: Limits| {
        Input::new("a.yaml", NESTED)
            .limits(limits)
            .place(&first())
            .map(|at| (at.line, at.column))
    };
    assert_eq!(at(Limits::NONE), Some((1, 4)));
    assert_eq!(at(tight()), None);
    let nodes = Limits {
        max_nodes: Some(3),
        ..Limits::NONE
    };
    assert_eq!(at(nodes), None);
}

#[test]
fn a_clone_tightened_after_the_tree_was_read_is_refused() {
    let loose = Input::new("a.yaml", NESTED);
    assert!(loose.place(&first()).is_some());
    let tightened = loose.clone().limits(tight());
    assert!(tightened.place(&first()).is_none());
    assert!(loose.place(&first()).is_some());
}

#[test]
fn a_refusal_ends_when_the_limits_that_gave_it_do() {
    static FORMAT: Counting = Counting::new();
    let refused = Input::new_with_format("a.count", NESTED, &FORMAT).limits(tight());
    assert!(refused.place(&first()).is_none());
    assert!(refused.place(&first()).is_none());
    assert_eq!(FORMAT.reads(), 1);
    let freed = refused.clone().limits(Limits::NONE);
    assert!(freed.place(&first()).is_some());
    assert!(refused.place(&first()).is_none());
}

#[test]
fn an_input_equals_its_clone_whether_or_not_it_was_placed_on() {
    let input = Input::new("a.yaml", "a: 1\n");
    let fresh = input.clone();
    assert!(input.place(&first()).is_some());
    assert_eq!(input, fresh);
    assert_ne!(input, Input::new("b.yaml", "a: 1\n"));
}

#[test]
fn a_format_is_compared_by_its_name() {
    static FORMAT: Counting = Counting::new();
    let yaml = Input::new("a.yaml", "a: 1\n");
    let counted = Input::new_with_format("a.yaml", "a: 1\n", &FORMAT);
    assert_ne!(yaml, counted);
    assert_eq!(yaml, counted.format(&YAML));
}

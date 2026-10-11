//! Fixture trees: one call per file, one temporary directory per suite and
//! per scenario.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "fixture")]

use std::panic::catch_unwind;

use ctl_core::fixture::Suite;
use ctl_core::indoc;

#[test]
fn a_nested_file_is_written_with_its_parents() {
    let suite = Suite::new("fixture");
    let root = suite.scenario("nested");
    let body = indoc! {"
        packages:
          - name: demo
    "};
    let path = root.file(".ctl/ver.yaml", body);

    assert_eq!(path, root.path().join(".ctl/ver.yaml"));
    assert_eq!(root.read(".ctl/ver.yaml"), body);
}

#[test]
fn each_scenario_owns_its_own_directory_inside_the_suite() {
    let suite = Suite::new("fixture");
    let first = suite.scenario("first");
    let second = suite.scenario("second");
    first.file("Cargo.toml", "[package]");

    assert_ne!(first.path(), second.path());
    assert!(first.path().starts_with(suite.root().path()));
    assert!(!second.path().join("Cargo.toml").exists());
}

#[test]
fn a_scenario_is_removed_when_it_drops() {
    let suite = Suite::new("fixture");
    let scenario = suite.scenario("dropped");
    let path = scenario.file("a.md", "a");
    drop(scenario);

    assert!(!path.exists());
    assert!(suite.root().path().exists());
}

#[test]
fn a_suite_is_removed_when_it_drops() {
    let suite = Suite::new("fixture");
    let path = suite.root().path().to_path_buf();
    drop(suite);

    assert!(!path.exists());
}

#[test]
fn a_path_outside_the_root_is_refused() {
    let suite = Suite::new("fixture");
    let root = suite.scenario("escape");
    let outside = suite.root().path().join("outside.md");

    assert!(catch_unwind(|| root.file("../outside.md", "x")).is_err());
    assert!(catch_unwind(|| root.file(&outside, "x")).is_err());
    assert!(!outside.exists());
}

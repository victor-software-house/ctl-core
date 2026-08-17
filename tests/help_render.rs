//! Styled help tables and `try_emit`.
#![allow(missing_docs)]
#![cfg(feature = "help")]

mod common;

use clap::CommandFactory;
use common::Toy;

#[test]
fn lists_commands() {
    let text = ctl_core::help::render(Toy::command());
    assert!(text.contains("Commands"));
    assert!(text.contains("status"));
}

#[test]
fn lists_output_flags() {
    let text = ctl_core::help::render(Toy::command());
    for needle in ["--format", "--color", "--no-color", "--quiet", "--dry-run"] {
        assert!(text.contains(needle), "missing {needle}");
    }
}

#[test]
fn try_emit_skips_without_help() {
    let args = ["toy", "status"].map(String::from);
    assert!(!ctl_core::help::try_emit_from::<Toy>(&args).unwrap());
}

#[test]
fn try_emit_runs_on_help() {
    let args = ["toy", "--help"].map(String::from);
    assert!(ctl_core::help::try_emit_from::<Toy>(&args).unwrap());
}

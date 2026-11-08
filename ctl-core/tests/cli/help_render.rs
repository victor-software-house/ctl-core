//! Help extraction through the public semantic document.
use clap::CommandFactory;
use ctl_core::{ColorMode, RenderOptions};

use crate::common::Toy;

fn help() -> String {
    ctl_core::help::document(Toy::command())
        .render(RenderOptions::new(ColorMode::Never).with_width(80))
}

#[test]
fn lists_commands() {
    let text = help();
    assert!(text.contains("Commands"));
    assert!(text.contains("status"));
}

#[test]
fn lists_output_flags() {
    let text = help();
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

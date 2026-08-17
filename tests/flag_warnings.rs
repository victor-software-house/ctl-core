//! Duplicate / contradictory chassis flags warn; last value still wins.
#![allow(missing_docs)]
#![cfg(feature = "cli")]

use ctl_core::{WarningKind, chassis_warnings, warn_opposites};

fn kinds(args: &[&str]) -> Vec<WarningKind> {
    chassis_warnings(args.iter().copied())
        .into_iter()
        .map(|warning| warning.kind)
        .collect()
}

#[test]
fn silent_when_flags_are_clean() {
    assert!(kinds(&["status", "--format", "json", "--color", "never"]).is_empty());
}

#[test]
fn repeated_format_warns() {
    assert_eq!(
        kinds(&["--format", "json", "--format", "pretty", "status"]),
        [WarningKind::Repeated]
    );
}

#[test]
fn color_and_no_color_are_contradictory() {
    let hits = kinds(&["--color", "always", "--no-color", "status"]);
    assert!(hits.contains(&WarningKind::Contradictory));
}

#[test]
fn dry_run_and_preview_are_redundant() {
    assert_eq!(
        kinds(&["--dry-run", "--preview", "status"]),
        [WarningKind::Redundant]
    );
}

#[test]
fn short_and_long_quiet_is_repeated() {
    assert_eq!(kinds(&["-q", "--quiet", "status"]), [WarningKind::Repeated]);
}

#[test]
fn opposite_pair_helper() {
    let hits = warn_opposites(["--pr", "--no-pr", "prepare"], &["--pr"], &["--no-pr"]);
    assert_eq!(hits[0].kind, WarningKind::Contradictory);
    assert!(warn_opposites(["--pr", "prepare"], &["--pr"], &["--no-pr"]).is_empty());
}

#[test]
fn warning_line_is_gnu_shaped() {
    let warning = chassis_warnings(["--color", "always", "--no-color"])
        .into_iter()
        .find(|warning| warning.kind == WarningKind::Contradictory);
    let Some(warning) = warning else {
        panic!("expected contradictory --color/--no-color");
    };
    let text = warning.line("toy");
    assert_eq!(
        text,
        "toy: warning: --color and --no-color both set; last wins"
    );
}

#[test]
fn mixed_clashes_are_all_reported() {
    let hits = kinds(&[
        "--format",
        "json",
        "--format",
        "pretty",
        "--color",
        "always",
        "--no-color",
        "--dry-run",
        "--preview",
        "status",
    ]);
    assert!(hits.contains(&WarningKind::Repeated));
    assert!(hits.contains(&WarningKind::Contradictory));
    assert!(hits.contains(&WarningKind::Redundant));
}

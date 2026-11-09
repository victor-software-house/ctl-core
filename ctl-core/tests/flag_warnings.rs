//! Duplicate / contradictory chassis flags warn; last value still wins.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "cli")]

use ctl_core::{FlagClash, chassis_warnings, warn_opposites};

fn clashes(args: &[&str]) -> Vec<FlagClash> {
    chassis_warnings(args.iter().copied())
        .into_iter()
        .map(|warning| warning.clash)
        .collect()
}

#[test]
fn silent_when_flags_are_clean() {
    assert_eq!(
        clashes(&["status", "--format", "json", "--color", "never"]),
        Vec::<FlagClash>::new()
    );
}

#[test]
fn repeated_format_warns() {
    assert_eq!(
        clashes(&["--format", "json", "--format", "pretty", "status"]),
        [FlagClash::Repeated]
    );
}

#[test]
fn color_and_no_color_are_contradictory() {
    let hits = clashes(&["--color", "always", "--no-color", "status"]);
    assert!(hits.contains(&FlagClash::Contradictory));
}

#[test]
fn dry_run_and_preview_are_redundant() {
    assert_eq!(
        clashes(&["--dry-run", "--preview", "status"]),
        [FlagClash::Redundant]
    );
}

#[test]
fn short_and_long_quiet_is_repeated() {
    assert_eq!(clashes(&["-q", "--quiet", "status"]), [FlagClash::Repeated]);
}

#[test]
fn opposite_pair_helper() {
    let hits = warn_opposites(["--pr", "--no-pr", "prepare"], &["--pr"], &["--no-pr"]);
    assert_eq!(hits[0].clash, FlagClash::Contradictory);
    assert_eq!(
        warn_opposites(["--pr", "prepare"], &["--pr"], &["--no-pr"]),
        Vec::<ctl_core::FlagWarning>::new()
    );
}

#[test]
fn warning_line_is_gnu_shaped() {
    let warning = chassis_warnings(["--color", "always", "--no-color"])
        .into_iter()
        .find(|warning| warning.clash == FlagClash::Contradictory);
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
    #[rustfmt::skip]
    let hits = clashes(&[
        "--format", "json",
        "--format", "pretty",
        "--color", "always",
        "--no-color",
        "--dry-run",
        "--preview",
        "status",
    ]);
    assert!(hits.contains(&FlagClash::Repeated));
    assert!(hits.contains(&FlagClash::Contradictory));
    assert!(hits.contains(&FlagClash::Redundant));
}

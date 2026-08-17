//! Flatten mixins: shorts, longs, `--preview`, `-c` ownership.
#![allow(missing_docs)]
#![cfg(feature = "cli")]

mod common;

use clap::Parser;
use common::{Toy, ToyCmd, parse};
use ctl_core::flags::{FormatArgs, switch};
use ctl_core::prelude::*;

#[test]
fn status_subcommand() {
    assert!(matches!(parse(&["status"]).command, ToyCmd::Status));
}

#[test]
fn format_short_and_long() {
    assert!(parse(&["-f", "json", "status"]).output.format.is_json());
    assert!(
        parse(&["--format", "json", "status"])
            .output
            .format
            .is_json()
    );
    assert!(
        !parse(&["--format", "pretty", "status"])
            .output
            .format
            .is_json()
    );
}

#[test]
fn color_short_long_and_negation() {
    assert_eq!(
        parse(&["-c", "never", "status"]).output.color(),
        ColorMode::Never
    );
    assert_eq!(
        parse(&["--color", "always", "status"]).output.color(),
        ColorMode::Always
    );
    assert_eq!(
        parse(&["--no-color", "status"]).output.color(),
        ColorMode::Never
    );
}

#[test]
fn quiet_and_dry_run() {
    assert!(parse(&["-q", "status"]).output.quiet);
    assert!(parse(&["--quiet", "status"]).output.quiet);
    assert!(parse(&["-n", "status"]).dry.dry_run);
    assert!(parse(&["--dry-run", "status"]).dry.dry_run);
    assert!(parse(&["--preview", "status"]).dry.dry_run);
}

#[test]
fn defaults() {
    let cli = parse(&["status"]);
    assert_eq!(cli.output.format, OutputFormat::Pretty);
    assert_eq!(cli.output.color(), ColorMode::Auto);
    assert!(!cli.output.quiet);
    assert!(!cli.dry.dry_run);
}

#[test]
fn all_long_and_all_short() {
    #[rustfmt::skip]
    let long = parse(&[
        "--format", "json",
        "--color", "never",
        "--quiet",
        "--dry-run",
        "status",
    ]);
    assert!(long.output.format.is_json() && long.output.quiet && long.dry.dry_run);
    assert_eq!(long.output.color(), ColorMode::Never);
    let short = parse(&["-f", "json", "-c", "never", "-q", "-n", "status"]);
    assert!(short.output.format.is_json() && short.output.quiet && short.dry.dry_run);
}

#[test]
fn help_and_version_are_clap_display() {
    for args in [["-h"], ["--help"], ["-V"], ["--version"]] {
        let err = Toy::try_parse_from(["toy"].into_iter().chain(args)).unwrap_err();
        assert!(matches!(
            err.kind(),
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
        ));
    }
}

#[test]
fn missing_command_is_help() {
    let err = Toy::try_parse_from(["toy"]).unwrap_err();
    assert_eq!(
        err.kind(),
        clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    );
}

#[test]
fn parser_debug_assert() {
    ctl_core::parser::verify::<Toy>();
}

#[test]
fn switch_follows_the_on_flag() {
    assert!(switch(true, false));
    assert!(!switch(false, true));
}

#[test]
fn last_format_wins() {
    assert_eq!(
        parse(&["--format", "json", "--format", "pretty", "status"])
            .output
            .format,
        OutputFormat::Pretty
    );
}

#[test]
fn dry_run_preview_pair_still_parses() {
    assert!(parse(&["--dry-run", "--preview", "status"]).dry.dry_run);
    assert!(parse(&["--preview", "--dry-run", "status"]).dry.dry_run);
}

#[derive(Parser, Debug)]
struct Split {
    #[command(flatten)]
    format: FormatArgs,
    #[command(flatten)]
    color: ColorLong,
    #[arg(short = 'c', long)]
    config: Option<String>,
}

#[test]
fn color_long_leaves_short_c_for_config() {
    let cli = Split::parse_from(["x", "-c", "cfg.toml", "--color", "never", "-f", "json"]);
    assert_eq!(cli.config.as_deref(), Some("cfg.toml"));
    assert_eq!(cli.color.color(), ColorMode::Never);
    assert!(cli.format.format.is_json());
}

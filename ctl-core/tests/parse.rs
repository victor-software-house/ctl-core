#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "cli")]

use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches, Parser};
use ctl_core::flags::{DryRunArgs, OutputArgs, switch};
use ctl_core::parser::apply_defaults;
use ctl_core::{ColorMode, OutputFormat};

#[derive(Parser, Debug)]
#[command(version, about = "toy", arg_required_else_help = true)]
struct Cli {
    #[command(flatten)]
    output: OutputArgs,
    #[command(flatten)]
    dry: DryRunArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    Prepare(PrepareArgs),
}

#[derive(clap::Args, Debug)]
struct PrepareArgs {
    #[arg(long, overrides_with = "no_pr")]
    pr: bool,
    #[arg(long, overrides_with = "pr")]
    no_pr: bool,
}

#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn parse(args: &[&str]) -> Cli {
    let mut words = vec!["toy"];
    words.extend_from_slice(args);
    let matches = apply_defaults(Cli::command())
        .try_get_matches_from(&words)
        .expect("parse");
    Cli::from_arg_matches(&matches).expect("from matches")
}

/// The error kind ctl-core's parser stops with for `args`.
fn stops_with(args: &[&str]) -> Option<ErrorKind> {
    let mut words = vec!["toy"];
    words.extend_from_slice(args);
    apply_defaults(Cli::command())
        .try_get_matches_from(&words)
        .err()
        .map(|error| error.kind())
}

/// `-h`, `--help`, and `-V` stop the parse with only the cli feature, so
/// ctl-core declares them itself and does not lean on Clap's `help`
/// feature.
#[test]
fn help_and_version_shorts() {
    assert_eq!(stops_with(&["-h"]), Some(ErrorKind::DisplayHelp));
    assert_eq!(stops_with(&["--help"]), Some(ErrorKind::DisplayHelp));
    assert_eq!(stops_with(&["prepare", "-h"]), Some(ErrorKind::DisplayHelp));
    assert_eq!(stops_with(&["-V"]), Some(ErrorKind::DisplayVersion));
}

/// `help <command>` selects ctl-core's `help` subcommand with only the cli
/// feature.
#[test]
fn help_subcommand_names_the_command() {
    let matches = apply_defaults(Cli::command())
        .try_get_matches_from(["toy", "help", "prepare"])
        .ok();
    let named = matches.as_ref().and_then(|matches| {
        let (help, below) = matches.subcommand()?;
        Some((help.to_owned(), below.subcommand_name()?.to_owned()))
    });
    assert_eq!(named, Some(("help".to_owned(), "prepare".to_owned())));
    let unknown = apply_defaults(Cli::command())
        .try_get_matches_from(["toy", "help", "unknown"])
        .err()
        .map(|error| error.kind());
    assert_eq!(unknown, Some(ErrorKind::InvalidSubcommand));
}

#[test]
fn format_and_color_short_long() {
    let cli = parse(&["-f", "json", "--color", "always", "prepare"]);
    assert_eq!(cli.output.format, OutputFormat::Json);
    assert_eq!(cli.output.color(), ColorMode::Always);
}

#[test]
fn no_color_negation() {
    let cli = parse(&["--color", "always", "--no-color", "prepare"]);
    assert_eq!(cli.output.color(), ColorMode::Never);
}

#[test]
fn dry_run_preview() {
    assert!(parse(&["-n", "prepare"]).dry.dry_run);
    assert!(parse(&["--dry-run", "prepare"]).dry.dry_run);
    assert!(parse(&["--preview", "prepare"]).dry.dry_run);
}

#[test]
fn pr_pair_last_wins() {
    match parse(&["prepare", "--pr"]).command {
        Command::Prepare(args) => assert!(switch(args.pr)),
    }
    match parse(&["prepare", "--pr", "--no-pr"]).command {
        Command::Prepare(args) => {
            assert!(!switch(args.pr));
            assert!(args.no_pr);
        }
    }
    match parse(&["prepare", "--no-pr", "--pr"]).command {
        Command::Prepare(args) => assert!(switch(args.pr)),
    }
}

#![allow(missing_docs)]
#![cfg(feature = "cli")]

use clap::Parser;
use ctl_core::flags::{DryRunArgs, OutputArgs, switch};
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

#[allow(clippy::expect_used)]
fn parse(args: &[&str]) -> Cli {
    let mut words = vec!["toy"];
    words.extend_from_slice(args);
    Cli::try_parse_from(words).expect("parse")
}

#[test]
fn help_and_version_shorts() {
    assert!(Cli::try_parse_from(["toy", "-h"]).is_err());
    let err = Cli::try_parse_from(["toy", "-h"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    let err = Cli::try_parse_from(["toy", "--help"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    let err = Cli::try_parse_from(["toy", "-V"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
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
        Command::Prepare(args) => assert!(switch(args.pr, args.no_pr)),
    }
    match parse(&["prepare", "--pr", "--no-pr"]).command {
        Command::Prepare(args) => {
            assert!(!switch(args.pr, args.no_pr));
            assert!(args.no_pr);
        }
    }
    match parse(&["prepare", "--no-pr", "--pr"]).command {
        Command::Prepare(args) => assert!(switch(args.pr, args.no_pr)),
    }
}

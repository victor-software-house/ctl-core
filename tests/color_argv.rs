//! Clap owns global output flag placement, attached values, and domain values.
#![allow(missing_docs)]
#![cfg(feature = "cli")]

use clap::{CommandFactory, FromArgMatches, Parser};
use ctl_core::{ColorMode, OutputArgs, OutputFormat};

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Status(StatusArgs),
}

#[derive(clap::Args)]
struct StatusArgs {
    #[arg(short = 'm', long, allow_hyphen_values = true)]
    message: Option<String>,
}

#[allow(clippy::expect_used)]
fn parse(args: &[&str]) -> Cli {
    let matches = ctl_core::parser::apply_defaults(Cli::command())
        .try_get_matches_from(args)
        .expect("parse test argv");
    Cli::from_arg_matches(&matches).expect("build typed CLI")
}

#[test]
fn attached_globals_work_after_subcommand() {
    let cli = parse(&["toy", "status", "-fjson", "-cnever"]);
    assert_eq!(cli.output.format, OutputFormat::Json);
    assert_eq!(cli.output.color(), ColorMode::Never);
}

#[test]
fn domain_hyphen_value_is_not_a_global_flag() {
    let cli = parse(&["toy", "status", "-m", "--format=json"]);
    assert_eq!(cli.output.format, OutputFormat::Pretty);
    match cli.command {
        Command::Status(args) => assert_eq!(args.message.as_deref(), Some("--format=json")),
    }
}

#[test]
fn global_values_are_last_wins() {
    let cli = parse(&[
        "toy",
        "--format",
        "pretty",
        "status",
        "--format=json",
        "--color",
        "always",
        "--no-color",
    ]);
    assert_eq!(cli.output.format, OutputFormat::Json);
    assert_eq!(cli.output.color(), ColorMode::Never);
}

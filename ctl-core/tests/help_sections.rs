//! The sections help draws beyond Clap's own: examples from `after_help`,
//! the long text on `--help`, visible aliases, and the command groups an
//! `App` names. Each page is a reviewed snapshot, at the 80-column fallback
//! and at its natural width.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "app")]

use std::iter;

use ctl_core::prelude::*;
use serde::Serialize;

#[path = "support/child.rs"]
mod child;

const EXAMPLES: &str = indoc! {"
    toy run --size 8x16 -- make test
    toy ls --json
"};

/// Run toy boxes.
///
/// Toy makes boxes, runs commands in them, and removes them when the command
/// ends. Every command prints a table, or JSON under `--json`.
#[derive(Parser)]
#[command(name = "toy", version = "1.2.3", after_help = EXAMPLES)]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    /// The daemon to talk to.
    ///
    /// A host name, with a port when the daemon does not listen on 4700.
    /// Without it, toy talks to the local socket.
    #[arg(long, global = true, value_name = "HOST")]
    host: Option<String>,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run one command in a new box.
    Run,
    /// Make a box.
    New,
    /// List the boxes.
    #[command(visible_alias = "list")]
    Ls,
    /// Remove a box.
    #[command(visible_alias = "remove")]
    Rm,
    /// Print the guide.
    Skill,
}

#[derive(Serialize)]
struct Done;

impl Present for Done {
    fn present(&self) -> Document {
        Document::new().paragraph("done")
    }
}

fn run(args: &[&str], width: Option<u16>) -> ExitCode {
    let app = App::<Toy>::new("toy")
        .command_group("Run work", ["run"])
        .command_group("Boxes", ["new", "ls", "rm"]);
    app.fallback_width(width)
        .view(|cli| cli.output.view())
        .run_from(iter::once("toy").chain(args.iter().copied()), |_| Ok(Done))
}

const CHILD: &str = "CTL_CORE_HELP_SECTIONS_CHILD";

/// The help `args` print, in a child at the 80-column fallback or at its
/// natural width.
fn help(test: &str, args: &[&str], width: Option<u16>) -> Option<String> {
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(child::case(|| run(args, width)), ExitCode::SUCCESS);
        return None;
    };
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    Some(child::case_stdout(&output))
}

#[test]
fn short_help_at_80_columns() {
    if let Some(page) = help("short_help_at_80_columns", &["-h"], Some(80)) {
        insta::assert_snapshot!("short_w80", page);
    }
}

#[test]
fn long_help_at_80_columns() {
    if let Some(page) = help("long_help_at_80_columns", &["--help"], Some(80)) {
        insta::assert_snapshot!("long_w80", page);
    }
}

#[test]
fn short_help_at_its_natural_width() {
    if let Some(page) = help("short_help_at_its_natural_width", &["-h"], None) {
        insta::assert_snapshot!("short_natural", page);
    }
}

#[test]
fn long_help_at_its_natural_width() {
    if let Some(page) = help("long_help_at_its_natural_width", &["--help"], None) {
        insta::assert_snapshot!("long_natural", page);
    }
}

#[test]
fn subcommand_help_lists_no_groups() {
    if let Some(page) = help("subcommand_help_lists_no_groups", &["help", "ls"], Some(80)) {
        assert!(!page.contains("Boxes"), "{page}");
        assert!(page.starts_with("Usage: toy ls"), "{page}");
    }
}

//! Help, version, and error output when Clap stops a parse: ctl-core renders
//! the help, so it needs none of Clap's help or color features. A parse error
//! reads `toy: <message>`, without Clap's `error:` prefix.
//!
//! The help is a reviewed snapshot in `tests/snapshots`. Clap's derive drops
//! the one trailing period of a doc comment's first paragraph, so the
//! snapshot reads "The row to show" for the doc "The row to show.".
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

#[derive(Parser)]
#[command(name = "toy", version = "1.2.3", about = "toy")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show one row.
    #[command(arg_required_else_help = true)]
    Show {
        /// The row to show.
        id: String,
    },
    /// Copy one row to another.
    Copy {
        /// The row to copy.
        from: String,
        /// The row to write.
        to: String,
    },
}

#[derive(Serialize)]
struct Shown {
    id: String,
}

impl Present for Shown {
    fn present(&self) -> Document {
        Document::new().paragraph(self.id.clone())
    }
}

fn run(args: &[&str]) -> ExitCode {
    App::<Toy>::new("toy")
        .view(|cli| cli.output.view())
        .run_from(iter::once("toy").chain(args.iter().copied()), |cli| {
            Ok(match cli.command {
                Cmd::Show { id } | Cmd::Copy { from: id, .. } => Shown { id },
            })
        })
}

const CHILD: &str = "CTL_CORE_CLAP_DISPLAY_CHILD";

/// The snapshots of a build with ctl-core's `error-context` are apart: Clap
/// then names the arguments and values in the text of a parse error, as it did
/// in any build that turned its own `error-context` on.
fn snapshots() -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    if cfg!(feature = "error-context") {
        settings.set_snapshot_suffix("error_context");
    }
    settings
}

/// The help of a build with `input-document` lists the two input flags, so its
/// snapshot is apart from the help of a build without them.
fn help_snapshots() -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    if cfg!(feature = "input-document") {
        settings.set_snapshot_suffix("input_document");
    }
    settings
}

#[test]
fn a_missing_argument_prints_the_subcommand_help_to_stderr() {
    let Some(output) = child::in_child(
        CHILD,
        "a_missing_argument_prints_the_subcommand_help_to_stderr",
        &[],
    ) else {
        assert_eq!(child::case(|| run(&["show"])), ExitCode::from(2));
        return;
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
    help_snapshots().bind(|| insta::assert_snapshot!("missing_argument", stderr));
}

#[test]
fn the_version_prints_clap_s_plain_line() {
    let Some(output) = child::in_child(CHILD, "the_version_prints_clap_s_plain_line", &[]) else {
        assert_eq!(child::case(|| run(&["--version"])), ExitCode::SUCCESS);
        return;
    };
    assert_eq!(child::case_stdout(&output), "toy 1.2.3\n");
}

#[test]
fn a_missing_argument_is_an_error_line_without_claps_prefix() {
    let Some(output) = child::in_child(
        CHILD,
        "a_missing_argument_is_an_error_line_without_claps_prefix",
        &[],
    ) else {
        assert_eq!(child::case(|| run(&["copy", "a"])), ExitCode::from(2));
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    snapshots().bind(|| insta::assert_snapshot!("parse_error_missing_argument", stderr));
}

#[test]
fn json_before_a_bad_argument_makes_the_error_json() {
    let Some(output) = child::in_child(
        CHILD,
        "json_before_a_bad_argument_makes_the_error_json",
        &[],
    ) else {
        assert_eq!(
            child::case(|| run(&["--json", "show", "--bogus"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    snapshots().bind(|| insta::assert_snapshot!("parse_error_json_flag", stderr));
}

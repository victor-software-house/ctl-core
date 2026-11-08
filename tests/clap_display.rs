//! Help and version output when Clap stops a parse: ctl-core renders the
//! help, so it needs none of Clap's help or color features.
//!
//! The help is a reviewed snapshot in `tests/snapshots`. Clap's derive drops
//! the one trailing period of a doc comment's first paragraph, so the
//! snapshot reads "The row to show" for the doc "The row to show.".
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "app")]

use std::process::{Command as Process, Output};

use ctl_core::prelude::*;
use serde::Serialize;

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
        .run_from(std::iter::once("toy").chain(args.iter().copied()), |cli| {
            Ok(match cli.command {
                Cmd::Show { id } => Shown { id },
            })
        })
}

const CHILD: &str = "CTL_CORE_CLAP_DISPLAY_CHILD";

/// Run `test` in a child and return its output, or `None` when this process
/// is that child and should run the case.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn in_child(test: &str) -> Option<Output> {
    if std::env::var_os(CHILD).as_deref() == Some(test.as_ref()) {
        return None;
    }
    let output = Process::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("FORCE_COLOR")
        .env_remove("NO_COLOR")
        .env_remove("COLORTERM")
        .output()
        .expect("spawn child");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Some(output)
}

#[test]
fn a_missing_argument_prints_the_subcommand_help_to_stderr() {
    let Some(output) = in_child("a_missing_argument_prints_the_subcommand_help_to_stderr") else {
        assert_eq!(run(&["show"]), ExitCode::from(2));
        return;
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
    insta::assert_snapshot!("missing_argument", stderr);
}

#[test]
fn the_version_prints_clap_s_plain_line() {
    let Some(output) = in_child("the_version_prints_clap_s_plain_line") else {
        assert_eq!(run(&["--version"]), ExitCode::SUCCESS);
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("toy 1.2.3\n"), "{stdout}");
    assert!(!stdout.contains('\u{1b}'), "{stdout}");
}

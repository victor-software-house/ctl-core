//! Where a failure goes under `--format json`: the envelope or the failed
//! result on stderr, and nothing on stdout. A handler's `CommandError` sets
//! the exit code and the envelope's kind.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "app")]

use std::iter;
use std::num::NonZeroU8;
use std::process::Output;

use anyhow::{Context as _, anyhow};
use ctl_core::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};

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
    Show {
        /// The row to show.
        id: String,
    },
    /// Fail with a plain error.
    Fail,
    /// Refuse with a kind and an exit code.
    Refuse,
    /// Report a result whose outcome is an error.
    Broken,
}

#[derive(Serialize)]
struct Report {
    id: String,
    failed: bool,
}

impl Present for Report {
    fn present(&self) -> Document {
        Document::new().paragraph(self.id.clone())
    }

    fn outcome(&self) -> Outcome {
        if self.failed {
            Outcome::Error
        } else {
            Outcome::Success
        }
    }
}

const NO_ROOM: NonZeroU8 = NonZeroU8::new(75).unwrap();

fn run(args: &[&str]) -> ExitCode {
    App::<Toy>::new("toy")
        .view(|cli| cli.output.view())
        .run_from(
            iter::once("toy").chain(args.iter().copied()),
            |cli| match cli.command {
                Cmd::Show { id } => Ok(Report { id, failed: false }),
                Cmd::Fail => Err(anyhow!("the row is locked")),
                Cmd::Refuse => Err(anyhow!("the budget is full")).context(
                    CommandError::new("no room for a box")
                        .kind("no_room")
                        .exit_code(NO_ROOM),
                ),
                Cmd::Broken => Ok(Report {
                    id: "broken".to_owned(),
                    failed: true,
                }),
            },
        )
}

const CHILD: &str = "CTL_CORE_JSON_FAILURES_CHILD";

/// The child's stderr parsed as JSON, after checking that stdout is empty
/// and that stderr holds no escape byte.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn stderr_json(output: &Output) -> Value {
    assert_eq!(child::case_stdout(output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
    serde_json::from_str(&stderr).expect("stderr holds one JSON document")
}

#[test]
fn emit_err_writes_the_envelope_to_stderr() {
    let Some(output) = child::in_child(CHILD, "emit_err_writes_the_envelope_to_stderr") else {
        let view = View::new(OutputFormat::Json, ColorMode::Never);
        assert_eq!(
            child::case(|| view.emit_err("toy", "nope").unwrap()),
            ExitCode::FAILURE
        );
        return;
    };
    assert_eq!(
        stderr_json(&output),
        json!({"status": "err", "schema_version": 1, "error": {"bin": "toy", "message": "nope"}})
    );
}

#[test]
fn a_parse_error_writes_the_envelope_to_stderr() {
    let Some(output) = child::in_child(CHILD, "a_parse_error_writes_the_envelope_to_stderr") else {
        assert_eq!(
            child::case(|| run(&["show", "--bogus", "--format", "json"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(
        stderr_json(&output),
        json!({
            "status": "err",
            "schema_version": 1,
            "error": {"bin": "toy", "message": "error: unexpected argument found", "kind": "usage"},
        })
    );
}

#[test]
fn an_error_outcome_writes_the_result_to_stderr() {
    let Some(output) = child::in_child(CHILD, "an_error_outcome_writes_the_result_to_stderr")
    else {
        assert_eq!(
            child::case(|| run(&["--format", "json", "broken"])),
            ExitCode::FAILURE
        );
        return;
    };
    assert_eq!(
        stderr_json(&output),
        json!({"id": "broken", "failed": true})
    );
}

#[test]
fn a_plain_handler_error_exits_1_with_no_kind() {
    let Some(output) = child::in_child(CHILD, "a_plain_handler_error_exits_1_with_no_kind") else {
        assert_eq!(
            child::case(|| run(&["--format", "json", "fail"])),
            ExitCode::FAILURE
        );
        return;
    };
    assert_eq!(
        stderr_json(&output),
        json!({
            "status": "err",
            "schema_version": 1,
            "error": {"bin": "toy", "message": "the row is locked"},
        })
    );
}

#[test]
fn a_command_error_sets_the_kind_and_the_exit() {
    let Some(output) = child::in_child(CHILD, "a_command_error_sets_the_kind_and_the_exit") else {
        assert_eq!(
            child::case(|| run(&["--format", "json", "refuse"])),
            ExitCode::from(75)
        );
        return;
    };
    assert_eq!(
        stderr_json(&output),
        json!({
            "status": "err",
            "schema_version": 1,
            "error": {
                "bin": "toy",
                "message": "no room for a box: the budget is full",
                "kind": "no_room",
            },
        })
    );
}

#[test]
fn a_command_error_sets_the_exit_of_pretty_output() {
    let Some(output) = child::in_child(CHILD, "a_command_error_sets_the_exit_of_pretty_output")
    else {
        assert_eq!(
            child::case(|| run(&["--color", "never", "refuse"])),
            ExitCode::from(75)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "toy: no room for a box: the budget is full\n"
    );
}

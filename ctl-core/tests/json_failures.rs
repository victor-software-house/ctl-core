//! Where a failure goes under `--format json`: the envelope or the failed
//! result on stderr, and nothing on stdout. Each part of a failure has a field
//! of its own, and a handler's `CommandError` sets the exit code and the
//! envelope's kind.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "app")]

use std::iter;
use std::num::NonZeroU8;
use std::process::Output;

use anyhow::{Context as _, anyhow};
use ctl_core::format::JSON;
#[cfg(feature = "input")]
use ctl_core::input::Input;
use ctl_core::prelude::*;
#[cfg(feature = "input")]
use ctl_core::validate::{at_least_one, cannot_be_empty};
use serde::Serialize;
use serde_json::{Value, json};

#[path = "support/child.rs"]
mod child;

#[derive(Parser)]
#[command(name = "toy", version = "1.2.3", about = "toy", env_prefix = "JSONTOY")]
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
    /// Fail with an error that has a cause.
    Read,
    /// Copy one row to another.
    Copy {
        /// The row to copy.
        from: String,
        /// The row to write.
        to: String,
    },
    /// Refuse an input with two problems.
    #[cfg(feature = "input")]
    Load,
    /// Refuse an input with one problem.
    #[cfg(feature = "input")]
    LoadOne,
    /// Refuse with a kind and an exit code.
    Refuse,
    /// Report a result whose outcome is an error.
    Broken,
}

#[cfg(feature = "input")]
#[derive(Debug, serde::Deserialize, garde::Validate)]
struct Config {
    #[garde(custom(cannot_be_empty("it names the release")))]
    name: String,
    #[garde(custom(at_least_one("label")))]
    labels: Vec<String>,
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
                Cmd::Show { id } | Cmd::Copy { from: id, .. } => Ok(Report { id, failed: false }),
                Cmd::Fail => Err(anyhow!("the row is locked")),
                Cmd::Read => Err(anyhow!("no such file")).context("reading config"),
                #[cfg(feature = "input")]
                Cmd::Load => Input::new("app.yaml", "name: \"\"\nlabels: []\n")
                    .load::<Config>()
                    .map(|_| Report {
                        id: "loaded".to_owned(),
                        failed: false,
                    })
                    .map_err(anyhow::Error::from),
                #[cfg(feature = "input")]
                Cmd::LoadOne => Input::new("app.yaml", "name: x\nlabels: []\n")
                    .load::<Config>()
                    .map(|_| Report {
                        id: "loaded".to_owned(),
                        failed: false,
                    })
                    .map_err(anyhow::Error::from),
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
    let Some(output) = child::in_child(CHILD, "emit_err_writes_the_envelope_to_stderr", &[]) else {
        let view = View::new(&JSON, ColorMode::Never);
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
fn an_unknown_argument() {
    let Some(output) = child::in_child(CHILD, "an_unknown_argument", &[]) else {
        assert_eq!(
            child::case(|| run(&["show", "--bogus", "--format", "json"])),
            ExitCode::from(2)
        );
        return;
    };
    let envelope = stderr_json(&output);
    let error = &envelope["error"];
    assert_eq!(error["kind"], "usage");
    assert_eq!(error["parse"]["reason"], "unknown_argument");
    assert_eq!(error["usage"], "toy show [OPTIONS] <ID>");
    let message = error["message"].as_str().unwrap();
    assert!(
        !message.contains('\n') && !message.contains("Usage"),
        "{message}"
    );
}

#[cfg(feature = "error-context")]
#[test]
fn the_parts_clap_s_error_context_gives() {
    let Some(output) = child::in_child(CHILD, "the_parts_clap_s_error_context_gives", &[]) else {
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
            "error": {
                "bin": "toy",
                "message": "unexpected argument '--bogus' found",
                "kind": "usage",
                "usage": "toy show [OPTIONS] <ID>",
                "parse": {
                    "reason": "unknown_argument",
                    "argument": "--bogus",
                    "tips": ["to pass '--bogus' as a value, use '-- --bogus'"],
                },
            },
        })
    );
}

#[cfg(feature = "error-context")]
#[test]
fn the_arguments_clap_found_missing() {
    let Some(output) = child::in_child(CHILD, "the_arguments_clap_found_missing", &[]) else {
        assert_eq!(
            child::case(|| run(&["copy", "a", "--format", "json"])),
            ExitCode::from(2)
        );
        return;
    };
    let envelope = stderr_json(&output);
    let parse = &envelope["error"]["parse"];
    assert_eq!(parse["reason"], "missing_required_argument");
    assert_eq!(parse["missing"], json!(["<TO>"]));
    assert_eq!(envelope["error"]["usage"], "toy copy [OPTIONS] <FROM> <TO>");
}

#[test]
fn a_parse_error_under_forced_colour_carries_no_escape_byte() {
    let Some(output) = child::in_child(
        CHILD,
        "a_parse_error_under_forced_colour_carries_no_escape_byte",
        &[("CLICOLOR_FORCE", "1"), ("FORCE_COLOR", "3")],
    ) else {
        assert_eq!(
            child::case(|| run(&["show", "--bogus", "--format", "json"])),
            ExitCode::from(2)
        );
        return;
    };
    let envelope = stderr_json(&output);
    assert_eq!(envelope["error"]["parse"]["reason"], "unknown_argument");
}

#[cfg(feature = "input")]
#[test]
fn a_refused_input() {
    let Some(output) = child::in_child(CHILD, "a_refused_input", &[]) else {
        assert_eq!(
            child::case(|| run(&["--format", "json", "load"])),
            ExitCode::FAILURE
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
                "message": "app.yaml has 2 problems",
                "problems": [
                    {
                        "input": "app.yaml",
                        "line": 2,
                        "column": 9,
                        "field": "labels",
                        "message": "must declare at least one label",
                    },
                    {
                        "input": "app.yaml",
                        "line": 1,
                        "column": 7,
                        "field": "name",
                        "message": "cannot be empty — it names the release",
                    },
                ],
            },
        })
    );
}

#[cfg(feature = "input")]
#[test]
fn a_refused_input_with_one_problem_is_still_a_summary() {
    let Some(output) = child::in_child(
        CHILD,
        "a_refused_input_with_one_problem_is_still_a_summary",
        &[],
    ) else {
        assert_eq!(
            child::case(|| run(&["--format", "json", "load-one"])),
            ExitCode::FAILURE
        );
        return;
    };
    let envelope = stderr_json(&output);
    assert_eq!(envelope["error"]["message"], "app.yaml has 1 problem");
    assert_eq!(
        envelope["error"]["problems"],
        json!([{
            "input": "app.yaml",
            "line": 2,
            "column": 9,
            "field": "labels",
            "message": "must declare at least one label",
        }])
    );
}

#[cfg(feature = "input")]
#[test]
fn a_refused_input_in_pretty() {
    let Some(output) = child::in_child(CHILD, "a_refused_input_in_pretty", &[]) else {
        assert_eq!(
            child::case(|| run(&["--color", "never", "--format", "pretty", "load"])),
            ExitCode::FAILURE
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "toy: app.yaml:2:9: labels: must declare at least one label\n\
         app.yaml:1:7: name: cannot be empty — it names the release\n"
    );
}

#[test]
fn a_handler_error_with_causes() {
    let Some(output) = child::in_child(CHILD, "a_handler_error_with_causes", &[]) else {
        assert_eq!(
            child::case(|| run(&["--format", "json", "read"])),
            ExitCode::FAILURE
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
                "message": "reading config",
                "causes": ["no such file"],
            },
        })
    );
}

#[test]
fn the_json_variable_makes_a_parse_error_json() {
    let Some(output) = child::in_child(
        CHILD,
        "the_json_variable_makes_a_parse_error_json",
        &[("JSONTOY_JSON", "true")],
    ) else {
        assert_eq!(child::case(|| run(&["show", "--bogus"])), ExitCode::from(2));
        return;
    };
    assert_eq!(stderr_json(&output)["error"]["kind"], "usage");
}

#[test]
fn an_error_outcome_writes_the_result_to_stderr() {
    let Some(output) = child::in_child(CHILD, "an_error_outcome_writes_the_result_to_stderr", &[])
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
    let Some(output) = child::in_child(CHILD, "a_plain_handler_error_exits_1_with_no_kind", &[])
    else {
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
    let Some(output) = child::in_child(CHILD, "a_command_error_sets_the_kind_and_the_exit", &[])
    else {
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
                "message": "no room for a box",
                "kind": "no_room",
                "causes": ["the budget is full"],
            },
        })
    );
}

#[test]
fn a_command_error_sets_the_exit_of_pretty_output() {
    let Some(output) =
        child::in_child(CHILD, "a_command_error_sets_the_exit_of_pretty_output", &[])
    else {
        assert_eq!(
            child::case(|| run(&["--color", "never", "--format", "pretty", "refuse"])),
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

#[test]
fn a_handler_error_in_pretty() {
    let Some(output) = child::in_child(CHILD, "a_handler_error_in_pretty", &[]) else {
        assert_eq!(
            child::case(|| run(&["--color", "never", "--format", "pretty", "read"])),
            ExitCode::FAILURE
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "toy: reading config: no such file\n"
    );
}

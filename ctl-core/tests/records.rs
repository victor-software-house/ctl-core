//! A command that returns records, run as a child process: each record
//! reaches stdout when the command produces it under `--stream`, and the
//! records are written as one result without it.
//!
//! The toy CLI has `status`, which returns one value, and `events`, which
//! returns a header record and records of an enum tagged with `type`. Its
//! `--mode` picks how the records are produced. A case that talks to the
//! running child, or closes its pipe, goes through `support/live.rs`; the
//! others run once and read the output, as `formats.rs` does.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "app")]

use std::io::{self, BufRead as _};
use std::num::NonZeroU8;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{iter, process};

use anyhow::anyhow;
use ctl_core::app::CommandError;
use ctl_core::format::{Format, FormatName, Formats, JSON, Output, Sink};
use ctl_core::prelude::*;
use ctl_core::view::{JsonLayout, Records, Reply};
use serde::ser::Error as _;
use serde::{Serialize, Serializer};
use serde_json::Value;

#[path = "support/child.rs"]
mod child;
#[path = "support/live.rs"]
mod live;

const CHILD: &str = "CTL_CORE_RECORDS_CHILD";

const YAML_NAME: FormatName = FormatName::new("yaml");

/// A machine format of the CLI's own with no stream form of its own: it
/// writes each record as it writes a whole result.
struct Yaml;

static YAML: Yaml = Yaml;

impl Format for Yaml {
    fn name(&self) -> FormatName {
        YAML_NAME
    }

    fn about(&self) -> &'static str {
        "Prefixed JSON"
    }

    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        use std::io::Write as _;

        write!(sink, "yaml:")?;
        serde_json::to_writer(&mut *sink, output.data())?;
        writeln!(sink)
    }
}

#[derive(Serialize)]
struct Status {
    pending: usize,
    #[serde(skip)]
    failing: bool,
}

impl Present for Status {
    fn present(&self) -> Document {
        Document::new().paragraph("status")
    }

    fn outcome(&self) -> Outcome {
        if self.failing {
            Outcome::Error
        } else {
            Outcome::Success
        }
    }

    fn exit_code(&self) -> u8 {
        if self.failing { 4 } else { 0 }
    }
}

/// Fails after the `type` field of its record was written, so the writer has
/// a partial line when it fails.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes a field to `serialize_with` by reference"
)]
fn unserializable<S: Serializer>(_: &usize, _: S) -> Result<S::Ok, S::Error> {
    Err(S::Error::custom("the record does not serialize"))
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event {
    Header {
        total: usize,
    },
    Tick {
        n: usize,
    },
    /// A record whose own outcome is an error and whose exit code is 5.
    Refused {
        n: usize,
    },
    /// A record that is a table of other columns than the others.
    Note {
        text: String,
    },
    /// A record that is fields, so it is no table.
    Detail {
        n: usize,
    },
    /// A record that is a table with no header row.
    Plain {
        n: usize,
    },
    /// A record that fails half way through its serialization.
    Broken {
        #[serde(serialize_with = "unserializable")]
        n: usize,
    },
}

/// A one-row table, with the same headers for every record that uses it, so
/// the collected records merge into one table.
fn counted(kind: &str, count: usize) -> Document {
    Document::new().table(Table::new(["type", "n"]).row([kind.to_owned(), count.to_string()]))
}

impl Present for Event {
    fn present(&self) -> Document {
        match self {
            Self::Header { total } => counted("header", *total),
            Self::Tick { n } => counted("tick", *n),
            Self::Refused { n } => counted("refused", *n),
            Self::Broken { n } => counted("broken", *n),
            Self::Note { text } => {
                Document::new().table(Table::new(["type", "text"]).row(["note", text.as_str()]))
            }
            Self::Detail { n } => {
                Document::new().fields(Fields::new().row("detail", n.to_string()))
            }
            Self::Plain { n } => {
                Document::new().table(Table::plain().row(["plain".to_owned(), n.to_string()]))
            }
        }
    }

    fn outcome(&self) -> Outcome {
        match self {
            Self::Refused { .. } => Outcome::Error,
            Self::Header { .. }
            | Self::Tick { .. }
            | Self::Broken { .. }
            | Self::Note { .. }
            | Self::Detail { .. }
            | Self::Plain { .. } => Outcome::Success,
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            Self::Refused { .. } => 5,
            Self::Header { .. }
            | Self::Tick { .. }
            | Self::Broken { .. }
            | Self::Note { .. }
            | Self::Detail { .. }
            | Self::Plain { .. } => 0,
        }
    }
}

#[derive(Parser)]
#[command(name = "toy", version, about = "toy", env_prefix = "TOY")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show one value.
    Status {
        /// Fail with its own exit code.
        #[arg(long)]
        failing: bool,
    },
    /// Show records.
    Events {
        /// How the records are produced: `fixed` (three), `lockstep` (each
        /// after a line on stdin), `endless`, `fail` (the third fails),
        /// `counted` (three, then a failure, counting each pull), `refused`
        /// (a record whose own outcome is an error), `broken` (a record that
        /// fails half way through its serialization), `alternate` (tables
        /// of two kinds, A, A, B, A), `between` (fields between two tables of
        /// one kind), `plain` (a table with no header row), or `refuse` (the
        /// handler fails).
        #[arg(long, default_value = "fixed")]
        mode: String,
    },
}

/// The records `counted` pulled, so a run that hides its output can show it
/// still pulled them all.
static PULLS: AtomicUsize = AtomicUsize::new(0);

const THREE: NonZeroU8 = NonZeroU8::MIN.saturating_add(2);

fn third_fails() -> anyhow::Error {
    CommandError::new("the third record failed")
        .kind("third")
        .exit_code(THREE)
        .into()
}

fn wait_for_line() {
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).ok();
}

fn events(mode: &str) -> anyhow::Result<Records<'static>> {
    let header = Event::Header { total: 3 };
    Ok(match mode {
        "fixed" => Records::new(header, (1..=3).map(|n| Ok(Event::Tick { n }))),
        "lockstep" => Records::new(
            header,
            (1..=3).map(|n| {
                wait_for_line();
                Ok(Event::Tick { n })
            }),
        ),
        "endless" => Records::new(header, (1..).map(|n| Ok(Event::Tick { n }))),
        "fail" => Records::new(
            header,
            (1..=3).map(|n| {
                if n == 3 {
                    Err(third_fails())
                } else {
                    Ok(Event::Tick { n })
                }
            }),
        ),
        "counted" => Records::new(
            header,
            (1..=4).map(|n| {
                PULLS.fetch_add(1, Ordering::SeqCst);
                if n == 4 {
                    Err(third_fails())
                } else {
                    Ok(Event::Tick { n })
                }
            }),
        ),
        "refused" => Records::new(
            header,
            [Event::Refused { n: 1 }, Event::Tick { n: 2 }]
                .into_iter()
                .map(Ok),
        ),
        "alternate" => Records::new(
            header,
            [
                Event::Tick { n: 1 },
                Event::Note {
                    text: "first".to_owned(),
                },
                Event::Tick { n: 2 },
            ]
            .into_iter()
            .map(Ok),
        ),
        "between" => Records::new(
            header,
            [Event::Detail { n: 1 }, Event::Tick { n: 2 }]
                .into_iter()
                .map(Ok),
        ),
        "plain" => Records::new(
            header,
            [Event::Plain { n: 1 }, Event::Plain { n: 2 }]
                .into_iter()
                .map(Ok),
        ),
        "broken" => Records::new(
            header,
            [
                Event::Tick { n: 1 },
                Event::Broken { n: 2 },
                Event::Tick { n: 3 },
            ]
            .into_iter()
            .map(Ok),
        ),
        _ => return Err(anyhow!("the toy refuses {mode}")),
    })
}

fn handle(toy: Toy) -> anyhow::Result<Reply<'static>> {
    Ok(match toy.command {
        Cmd::Status { failing } => Reply::value(Status {
            pending: 2,
            failing,
        }),
        Cmd::Events { mode } => events(&mode)?.into(),
    })
}

fn app(formats: Formats) -> App<Toy> {
    App::<Toy>::new("toy")
        .formats(formats)
        .view(|toy| toy.output.view())
}

fn run_app(app: App<Toy>, args: &[&str]) -> ExitCode {
    app.run_from(iter::once("toy").chain(args.iter().copied()), handle)
}

fn run(args: &[&str]) -> ExitCode {
    run_app(app(Formats::builtin().add(&YAML)), args)
}

/// The lines of `text`, each parsed as one JSON value.
fn json_lines(text: &str) -> Vec<Value> {
    text.lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|error| panic!("{line}: {error}")))
        .collect()
}

fn has_string_type(value: &Value) -> bool {
    matches!(value.get("type"), Some(Value::String(_)))
}

/// Answer each record the child writes with a line on its stdin, so the child
/// produces the next one only then, and return the lines read. A run that
/// collects its records before it writes the first never answers, and the
/// read ends at the deadline.
fn lockstep(test: &str, args: &[&str], vars: &[(&str, &str)]) -> Vec<String> {
    lockstep_lines(test, args, vars, [1, 1])
}

/// [`lockstep`] for a format that writes `first` lines for the header
/// record, then `each` lines for each record after it.
fn lockstep_lines(
    test: &str,
    args: &[&str],
    vars: &[(&str, &str)],
    [first, each]: [usize; 2],
) -> Vec<String> {
    let Some(mut live) = live::spawn(CHILD, test, vars) else {
        live::serve(ExitCode::SUCCESS, || run(args));
    };
    let mut lines: Vec<String> = (0..first).map(|_| live.read_line()).collect();
    for _ in 0..3 {
        live.write_line("go");
        lines.extend((0..each).map(|_| live.read_line()));
    }
    let ended = live.finish();
    assert!(ended.status.success(), "{}", ended.stderr);
    assert_eq!(ended.stderr, "");
    lines
}

#[test]
fn json_records_arrive_one_line_at_a_time() {
    let lines = lockstep(
        "json_records_arrive_one_line_at_a_time",
        &["events", "--mode", "lockstep", "--stream", "--json"],
        &[],
    );
    let records = json_lines(&lines.join("\n"));
    assert!(records.iter().all(has_string_type), "{lines:?}");
    assert_eq!(records.len(), 4);
}

#[test]
fn tsv_writes_its_header_and_first_row_then_one_line_per_record() {
    let lines = lockstep_lines(
        "tsv_writes_its_header_and_first_row_then_one_line_per_record",
        &[
            "events", "--mode", "lockstep", "--stream", "--format", "tsv",
        ],
        &[],
        [2, 1],
    );
    assert_eq!(
        lines,
        ["type\tn", "header\t3", "tick\t1", "tick\t2", "tick\t3"]
    );
}

#[test]
fn pretty_writes_each_record_before_the_next_is_produced() {
    let lines = lockstep_lines(
        "pretty_writes_each_record_before_the_next_is_produced",
        &[
            "events", "--mode", "lockstep", "--stream", "--format", "pretty",
        ],
        &[],
        [5, 6],
    );
    let blank: Vec<_> = lines.iter().filter(|line| line.is_empty()).collect();
    assert_eq!(blank.len(), 3, "{lines:?}");
    assert_eq!(lines.len(), 23, "{lines:?}");
}

#[test]
fn a_format_with_no_stream_form_writes_each_record_as_it_arrives() {
    let lines = lockstep(
        "a_format_with_no_stream_form_writes_each_record_as_it_arrives",
        &[
            "events", "--mode", "lockstep", "--stream", "--format", "yaml",
        ],
        &[],
    );
    assert!(
        lines.iter().all(|line| line.starts_with("yaml:{\"type\":")),
        "{lines:?}"
    );
}

#[test]
fn an_endless_stream_ends_quietly_when_the_reader_closes_the_pipe() {
    let test = "an_endless_stream_ends_quietly_when_the_reader_closes_the_pipe";
    let Some(mut live) = live::spawn(CHILD, test, &[]) else {
        live::serve(ExitCode::SUCCESS, || {
            run(&["events", "--mode", "endless", "--stream", "--json"])
        });
    };
    let first = live.read_line();
    assert!(first.contains("\"header\""), "{first}");
    let ended = live.finish();
    assert!(ended.status.success(), "{}", ended.stderr);
    assert_eq!(ended.stderr, "");
}

#[test]
fn a_variable_streams_the_records() {
    let lines = lockstep(
        "a_variable_streams_the_records",
        &["events", "--mode", "lockstep", "--json"],
        &[("TOY_STREAM", "true")],
    );
    assert_eq!(lines.len(), 4);
}

#[test]
fn whole_json_is_one_array_of_typed_records() {
    let Some(output) = child::in_child(CHILD, "whole_json_is_one_array_of_typed_records", &[])
    else {
        assert_eq!(
            child::case(|| run(&["events", "--json"])),
            ExitCode::SUCCESS
        );
        return;
    };
    let records: Vec<Value> = serde_json::from_str(&child::case_stdout(&output)).unwrap();
    assert_eq!(records.len(), 4);
    assert!(records.iter().all(has_string_type));
}

/// The third record fails with exit code 3: what was pulled stays on stdout
/// and the envelope goes to stderr.
fn failed_envelope(output: &process::Output) {
    failed_stderr(&String::from_utf8_lossy(&output.stderr));
}

/// [`failed_envelope`] for the text of stderr.
fn failed_stderr(stderr: &str) {
    let envelope: Value =
        serde_json::from_str(stderr).unwrap_or_else(|error| panic!("{stderr}: {error}"));
    let text = envelope.to_string();
    assert!(text.contains("the third record failed"), "{text}");
    assert!(text.contains("\"third\""), "{text}");
}

#[test]
fn the_third_record_fails_and_the_collected_array_is_closed() {
    let test = "the_third_record_fails_and_the_collected_array_is_closed";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["events", "--mode", "fail", "--json"])),
            ExitCode::from(3)
        );
        return;
    };
    let records: Vec<Value> = serde_json::from_str(&child::case_stdout(&output)).unwrap();
    assert_eq!(records.len(), 3);
    failed_envelope(&output);
}

#[test]
fn the_third_record_fails_and_the_streamed_lines_stay() {
    let test = "the_third_record_fails_and_the_streamed_lines_stay";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["events", "--mode", "fail", "--stream", "--json"])),
            ExitCode::from(3)
        );
        return;
    };
    assert_eq!(json_lines(&child::case_stdout(&output)).len(), 3);
    failed_envelope(&output);
}

/// A child cannot have a terminal, so this proves the stream default beats
/// the piped default, `tsv`; the terminal default is `pretty`, which the
/// stream default beats the same way.
#[test]
fn stream_alone_writes_json_lines_on_a_pipe() {
    let Some(output) = child::in_child(CHILD, "stream_alone_writes_json_lines_on_a_pipe", &[])
    else {
        assert_eq!(
            child::case(|| run(&["events", "--stream"])),
            ExitCode::SUCCESS
        );
        return;
    };
    let records = json_lines(&child::case_stdout(&output));
    assert_eq!(records.len(), 4);
    assert!(records.iter().all(has_string_type));
}

#[test]
fn a_single_value_streams_as_one_compact_line_with_no_type() {
    let test = "a_single_value_streams_as_one_compact_line_with_no_type";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["status", "--stream", "--json"])),
            ExitCode::SUCCESS
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "{\"pending\":2}\n");
}

#[test]
fn a_failed_value_under_stream_goes_to_stderr_with_its_own_code() {
    let test = "a_failed_value_under_stream_goes_to_stderr_with_its_own_code";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["status", "--failing", "--stream", "--json"])),
            ExitCode::from(4)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("\"pending\""), "{stderr}");
}

#[test]
fn a_pretty_layout_still_streams_one_record_per_line() {
    let test = "a_pretty_layout_still_streams_one_record_per_line";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        let app = app(Formats::builtin()).json_layout(JsonLayout::Pretty);
        assert_eq!(
            child::case(|| run_app(app, &["events", "--stream", "--json"])),
            ExitCode::SUCCESS
        );
        return;
    };
    assert_eq!(json_lines(&child::case_stdout(&output)).len(), 4);
}

#[test]
fn quiet_hides_the_stream_default_and_still_drains_the_records() {
    let test = "quiet_hides_the_stream_default_and_still_drains_the_records";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        let code = child::case(|| run(&["events", "--mode", "counted", "--stream", "--quiet"]));
        assert_eq!(code, ExitCode::from(3));
        assert_eq!(PULLS.load(Ordering::SeqCst), 4);
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    failed_envelope(&output);
}

#[test]
fn quiet_does_not_hide_a_named_machine_format_under_stream() {
    let test = "quiet_does_not_hide_a_named_machine_format_under_stream";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["events", "--stream", "--json", "--quiet"])),
            ExitCode::SUCCESS
        );
        return;
    };
    assert_eq!(json_lines(&child::case_stdout(&output)).len(), 4);
}

#[test]
fn a_parse_error_under_bare_stream_is_a_json_error() {
    let test = "a_parse_error_under_bare_stream_is_a_json_error";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["events", "--stream", "--bogus"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let envelope: Value =
        serde_json::from_str(&stderr).unwrap_or_else(|error| panic!("{stderr}: {error}"));
    assert!(envelope.to_string().contains("\"usage\""), "{stderr}");
}

#[test]
fn a_handler_error_under_bare_stream_is_a_json_error() {
    let test = "a_handler_error_under_bare_stream_is_a_json_error";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["events", "--mode", "refuse", "--stream"])),
            ExitCode::FAILURE
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let envelope: Value =
        serde_json::from_str(&stderr).unwrap_or_else(|error| panic!("{stderr}: {error}"));
    assert!(
        envelope.to_string().contains("the toy refuses refuse"),
        "{stderr}"
    );
}

#[test]
fn a_removed_stream_default_is_a_usage_error_that_lists_the_registered_names() {
    let test = "a_removed_stream_default_is_a_usage_error_that_lists_the_registered_names";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        let app = app(Formats::builtin().remove(FormatName::JSON));
        assert_eq!(
            child::case(|| run_app(app, &["events", "--stream"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let unwrapped = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        unwrapped.contains(
            "the stream default `json` is not registered; use one of pretty, tsv, csv, markdown"
        ),
        "{stderr}"
    );
}

/// `jsonl` is not a format: the stream form of `json` is one line per record.
/// This passed before records existed too, and stays as a guard against adding
/// the name.
#[test]
fn there_is_no_jsonl_format() {
    let Some(output) = child::in_child(CHILD, "there_is_no_jsonl_format", &[]) else {
        assert_eq!(
            child::case(|| run(&["status", "--format", "jsonl"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown format `jsonl`"), "{stderr}");
}

/// The records of `mode`, for a test that shows them without `App`.
fn records(mode: &str) -> Records<'static> {
    events(mode).unwrap_or_else(|error| panic!("{error}"))
}

/// The stdout of a child that wrote the records of `test` as one run.
fn written(test: &str, args: &'static [&'static str], vars: &[(&str, &str)]) -> Option<String> {
    let Some(output) = child::in_child(CHILD, test, vars) else {
        assert_eq!(child::case(|| run(args)), ExitCode::SUCCESS);
        return None;
    };
    Some(child::case_stdout(&output))
}

#[test]
fn a_closed_stdout_does_not_hide_a_failed_iterator() {
    let test = "a_closed_stdout_does_not_hide_a_failed_iterator";
    let Some(mut live) = live::spawn(CHILD, test, &[]) else {
        live::serve(ExitCode::from(3), || {
            wait_for_line();
            run(&["events", "--mode", "fail", "--json"])
        });
    };
    live.close_stdout();
    live.write_line("go");
    let ended = live.finish();
    assert!(ended.status.success(), "{}", ended.stderr);
    failed_stderr(&ended.stderr);
}

#[test]
fn a_record_that_fails_to_serialize_leaves_no_partial_line() {
    let test = "a_record_that_fails_to_serialize_leaves_no_partial_line";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["events", "--mode", "broken", "--stream", "--json"])),
            ExitCode::FAILURE
        );
        return;
    };
    let stdout = child::case_stdout(&output);
    assert!(stdout.ends_with('\n'), "{stdout:?}");
    assert_eq!(json_lines(&stdout).len(), 2, "{stdout:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("the record does not serialize"), "{stderr}");
}

#[test]
fn a_collected_record_that_fails_to_serialize_writes_its_error_and_no_output() {
    let test = "a_collected_record_that_fails_to_serialize_writes_its_error_and_no_output";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["events", "--mode", "broken", "--json"])),
            ExitCode::FAILURE
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("the record does not serialize"), "{stderr}");
}

#[test]
fn a_parse_error_after_the_bad_argument_still_sees_stream() {
    let test = "a_parse_error_after_the_bad_argument_still_sees_stream";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        assert_eq!(
            child::case(|| run(&["events", "--bogus", "--stream"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let envelope: Value =
        serde_json::from_str(&stderr).unwrap_or_else(|error| panic!("{stderr}: {error}"));
    assert!(envelope.to_string().contains("\"usage\""), "{stderr}");
}

#[test]
fn a_record_whose_outcome_is_an_error_is_collected_to_stdout_and_exits_zero() {
    let test = "a_record_whose_outcome_is_an_error_is_collected_to_stdout_and_exits_zero";
    let Some(stdout) = written(test, &["events", "--mode", "refused", "--json"], &[]) else {
        return;
    };
    let records: Vec<Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(records.len(), 3, "{stdout}");
}

#[test]
fn a_record_whose_outcome_is_an_error_is_streamed_to_stdout_and_exits_zero() {
    let test = "a_record_whose_outcome_is_an_error_is_streamed_to_stdout_and_exits_zero";
    let Some(stdout) = written(
        test,
        &["events", "--mode", "refused", "--stream", "--json"],
        &[],
    ) else {
        return;
    };
    assert_eq!(json_lines(&stdout).len(), 3, "{stdout}");
}

#[test]
fn collected_records_of_one_table_each_are_one_table_in_tsv() {
    let test = "collected_records_of_one_table_each_are_one_table_in_tsv";
    if let Some(stdout) = written(test, &["events", "--format", "tsv"], &[]) {
        insta::assert_snapshot!("records_tsv", stdout);
    }
}

#[test]
fn collected_records_of_one_table_each_are_one_table_in_pretty() {
    let test = "collected_records_of_one_table_each_are_one_table_in_pretty";
    if let Some(stdout) = written(test, &["events", "--format", "pretty"], &[]) {
        insta::assert_snapshot!("records_pretty", stdout);
    }
}

#[test]
fn show_records_writes_the_records_without_app() {
    let test = "show_records_writes_the_records_without_app";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        let code =
            child::case(|| View::new(&JSON, ColorMode::Never).show_records(records("fixed")));
        assert!(matches!(code, Ok(code) if code == ExitCode::SUCCESS));
        return;
    };
    let written: Vec<Value> = serde_json::from_str(&child::case_stdout(&output)).unwrap();
    assert_eq!(written.len(), 4);
}

#[test]
fn show_records_returns_the_error_of_the_iterator_without_app() {
    let test = "show_records_returns_the_error_of_the_iterator_without_app";
    let Some(output) = child::in_child(CHILD, test, &[]) else {
        let result = child::case(|| {
            View::new(&JSON, ColorMode::Never)
                .stream(true)
                .show_records(records("fail"))
        });
        let Err(error) = result else {
            panic!("the iterator failed and the run did not");
        };
        let failure = error.downcast_ref::<CommandError>();
        assert_eq!(
            failure.map(ToString::to_string).as_deref(),
            Some("the third record failed")
        );
        return;
    };
    assert_eq!(json_lines(&child::case_stdout(&output)).len(), 3);
}

#[test]
fn a_variable_names_the_format_under_stream() {
    let test = "a_variable_names_the_format_under_stream";
    let Some(stdout) = written(test, &["events", "--stream"], &[("TOY_FORMAT", "yaml")]) else {
        return;
    };
    assert_eq!(stdout.lines().count(), 4, "{stdout}");
    assert!(
        stdout
            .lines()
            .all(|line| line.starts_with("yaml:{\"type\":")),
        "{stdout}"
    );
}

#[test]
fn the_json_flag_beats_a_format_variable_under_stream() {
    let test = "the_json_flag_beats_a_format_variable_under_stream";
    let Some(stdout) = written(
        test,
        &["events", "--stream", "--json"],
        &[("TOY_FORMAT", "yaml")],
    ) else {
        return;
    };
    assert_eq!(json_lines(&stdout).len(), 4, "{stdout}");
}

#[test]
fn forced_colour_never_reaches_json_lines() {
    let test = "forced_colour_never_reaches_json_lines";
    let vars = [
        ("CLICOLOR_FORCE", "1"),
        ("FORCE_COLOR", "3"),
        ("COLORTERM", "truecolor"),
    ];
    let Some(stdout) = written(test, &["events", "--stream", "--json"], &vars) else {
        return;
    };
    assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
    assert_eq!(json_lines(&stdout).len(), 4);
}

/// The formats whose stream form is written for `--stream`.
const BUILT_IN: [&str; 4] = ["tsv", "csv", "markdown", "pretty"];

/// One snapshot per built-in format of the records of `$mode` under
/// `--stream`. A CSV snapshot shows each carriage return as `<CR>`, since
/// insta normalizes line breaks.
macro_rules! stream_snapshots {
    ($($test:ident: $mode:literal;)*) => {$(
        #[test]
        fn $test() {
            for format in BUILT_IN {
                let args = &["events", "--mode", $mode, "--stream"];
                let Some(stdout) = written(stringify!($test), args, &[("TOY_FORMAT", format)])
                else {
                    return;
                };
                insta::assert_snapshot!(
                    format!("{}_{format}", stringify!($test)),
                    stdout.replace('\r', "<CR>")
                );
            }
        }
    )*};
}

stream_snapshots! {
    stream_of_records_of_one_table: "fixed";
    stream_of_records_of_two_tables_that_alternate: "alternate";
    stream_of_fields_between_two_tables_of_one_header: "between";
    stream_of_a_plain_table: "plain";
}

#[test]
fn a_streamed_table_is_the_collected_table_in_tsv_csv_and_markdown() {
    let test = "a_streamed_table_is_the_collected_table_in_tsv_csv_and_markdown";
    for format in ["tsv", "csv", "markdown"] {
        let Some(streamed) = written(
            test,
            &["events"],
            &[("TOY_STREAM", "true"), ("TOY_FORMAT", format)],
        ) else {
            return;
        };
        let Some(collected) = written(test, &["events"], &[("TOY_FORMAT", format)]) else {
            return;
        };
        assert_eq!(streamed, collected, "{format}");
    }
}

#[test]
fn forced_colour_never_reaches_a_tabular_stream() {
    let test = "forced_colour_never_reaches_a_tabular_stream";
    for format in ["tsv", "csv", "markdown"] {
        let vars = [
            ("CLICOLOR_FORCE", "1"),
            ("FORCE_COLOR", "3"),
            ("COLORTERM", "truecolor"),
            ("TOY_FORMAT", format),
        ];
        let Some(stdout) = written(test, &["events", "--stream"], &vars) else {
            return;
        };
        assert!(!stdout.contains('\u{1b}'), "{format}: {stdout:?}");
        assert!(stdout.contains("tick"), "{format}: {stdout:?}");
    }
}

#[test]
fn a_pretty_stream_writes_colour_when_asked() {
    let test = "a_pretty_stream_writes_colour_when_asked";
    let args = &[
        "events", "--stream", "--format", "pretty", "--color", "always",
    ];
    let Some(stdout) = written(test, args, &[]) else {
        return;
    };
    assert!(stdout.contains('\u{1b}'), "{stdout:?}");
}

#[test]
fn a_pretty_stream_writes_no_colour_when_refused() {
    let test = "a_pretty_stream_writes_no_colour_when_refused";
    let args = &[
        "events", "--stream", "--format", "pretty", "--color", "never",
    ];
    let vars = [("CLICOLOR_FORCE", "1"), ("FORCE_COLOR", "3")];
    let Some(stdout) = written(test, args, &vars) else {
        return;
    };
    assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
}

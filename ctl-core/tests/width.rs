//! The width and colour of pretty output, decided for the stream it is
//! written to: the presentation scenarios of the render-engine change.
//!
//! Each case runs `App` in a child process whose streams and environment the
//! test sets, since the decision reads both. In the child, stdout is a pipe;
//! the narrow-stderr case puts stderr on a terminal of its own.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(unix, feature = "app"))]

use std::io::Read as _;
use std::process::{Command as Process, Stdio};
use std::sync::mpsc;
use std::time::Duration;
use std::{env, iter, thread};

use ctl_core::prelude::*;
use pty_process::{Size, blocking};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "toy", version, about = "toy")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print a rule, a paragraph long enough to wrap at any terminal width,
    /// and a table whose one row is wider than the fallback width.
    Show,
}

const PARAGRAPH: &str = "Move the parser onto the shared input loader so every \
                         ledger problem is reported with its file and line, then \
                         remove the old schema module and its tests, and record \
                         the change in the changelog before the next release.";

const TITLE: &str = "Document the retry rules for the publish step, including the long wait \
                     after a rate limit and the backoff after a network failure";

#[derive(Serialize)]
struct Shown;

impl Present for Shown {
    fn present(&self) -> Document {
        Document::new()
            .rule(None)
            .paragraph(PARAGRAPH)
            .table(Table::new(["id", "title"]).row(["TSK-019", TITLE]))
    }
}

/// The variable that marks a process as the child of the named case.
const CHILD: &str = "CTL_CORE_WIDTH_CHILD";

/// The lines the child prints around the output under test, so the test
/// harness's own lines on stdout are left out.
const START: &str = "<<<";
const END: &str = ">>>";

/// Runs `App` on `args` with `configure` applied, between the two markers.
#[expect(
    clippy::print_stdout,
    reason = "the markers frame the output under test on the child's stdout"
)]
fn run(args: &[&str], configure: impl FnOnce(App<Toy>) -> App<Toy>) -> ExitCode {
    println!("{START}");
    let code = configure(App::<Toy>::new("toy").view(|cli| cli.output.view())).run_from(
        iter::once("toy").chain(args.iter().copied()),
        |cli| match cli.command {
            Cmd::Show => Ok(Shown),
        },
    );
    println!("{END}");
    code
}

/// This test binary, running only `test`, with no variable of the parent's
/// that could change a width, a colour, or a style.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn child(test: &str, variables: &[(&str, &str)]) -> Process {
    let mut command = Process::new(env::current_exe().expect("test binary"));
    command
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, test);
    for name in [
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "NO_COLOR",
        "COLORTERM",
        "COLUMNS",
        DEFAULT_COLUMN_BUFFER_ENV,
        DEFAULT_RECORD_STYLE_ENV,
        DEFAULT_LIST_STYLE_ENV,
        DEFAULT_ROW_SEPARATION_ENV,
    ] {
        command.env_remove(name);
    }
    command.env("TERM", "xterm");
    for (name, value) in variables {
        command.env(name, value);
    }
    command
}

fn is_child(test: &str) -> bool {
    env::var_os(CHILD).as_deref() == Some(test.as_ref())
}

/// The lines the child's `App` wrote to stdout, between the markers.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn rendered(test: &str, variables: &[(&str, &str)]) -> Vec<String> {
    let output = child(test, variables).output().expect("spawn child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
        .lines()
        .skip_while(|line| !line.ends_with(START))
        .skip(1)
        .take_while(|line| *line != END)
        .map(str::to_owned)
        .collect()
}

fn width(line: &str) -> usize {
    line.chars().count()
}

/// The rule, the first line, which fills the width laid out to.
fn rule_width(lines: &[String]) -> usize {
    let rule = lines.first().map_or("", String::as_str);
    assert!(rule.chars().all(|character| character == '─'), "{lines:#?}");
    width(rule)
}

/// `text` without its SGR sequences.
fn plain(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut escape = false;
    for character in text.chars() {
        match (escape, character) {
            (false, '\u{1b}') => escape = true,
            (false, _) => plain.push(character),
            (true, 'm') => escape = false,
            (true, _) => {}
        }
    }
    plain
}

#[test]
fn a_detected_width_keeps_the_buffer() {
    const TEST: &str = "a_detected_width_keeps_the_buffer";
    if is_child(TEST) {
        assert_eq!(run(&["show"], |app| app), ExitCode::SUCCESS);
        return;
    }
    let lines = rendered(TEST, &[("COLUMNS", "119")]);
    assert_eq!(rule_width(&lines), 117, "{lines:#?}");
    assert!(lines.iter().all(|line| width(line) <= 117), "{lines:#?}");
}

#[test]
fn an_explicit_width_is_exact() {
    const TEST: &str = "an_explicit_width_is_exact";
    if is_child(TEST) {
        let code = run(&["show"], |app| {
            app.view(|cli| cli.output.view().width(100))
        });
        assert_eq!(code, ExitCode::SUCCESS);
        return;
    }
    let lines = rendered(TEST, &[("COLUMNS", "119")]);
    assert_eq!(rule_width(&lines), 100, "{lines:#?}");
    assert!(lines.iter().all(|line| width(line) <= 100), "{lines:#?}");
}

#[test]
fn everything_piped_lays_out_to_the_fallback() {
    const TEST: &str = "everything_piped_lays_out_to_the_fallback";
    if is_child(TEST) {
        assert_eq!(run(&["show"], |app| app), ExitCode::SUCCESS);
        return;
    }
    let lines = rendered(TEST, &[]);
    assert_eq!(rule_width(&lines), 80, "{lines:#?}");
    assert!(lines.iter().all(|line| width(line) <= 80), "{lines:#?}");
}

#[test]
fn no_fallback_lays_a_table_out_at_its_natural_width() {
    const TEST: &str = "no_fallback_lays_a_table_out_at_its_natural_width";
    if is_child(TEST) {
        let code = run(&["show"], |app| app.fallback_width(None));
        assert_eq!(code, ExitCode::SUCCESS);
        return;
    }
    let lines = rendered(TEST, &[]);
    assert!(
        lines.iter().any(|line| line.contains(TITLE)),
        "the title should keep one line: {lines:#?}"
    );
}

/// stdout is a pipe and stderr a terminal 50 columns wide, so help printed
/// after a usage error asks stderr for its width and its colour.
#[test]
fn help_on_a_narrow_stderr_fits_stderr() {
    const TEST: &str = "help_on_a_narrow_stderr_fits_stderr";
    if is_child(TEST) {
        assert_eq!(run(&[], |app| app), ExitCode::from(2));
        return;
    }
    let (pty, pts) = blocking::open().unwrap_or_else(|error| panic!("open a terminal: {error}"));
    pty.resize(Size::new(24, 50))
        .unwrap_or_else(|error| panic!("size the terminal: {error}"));
    let mut command = blocking::Command::new(
        env::current_exe().unwrap_or_else(|error| panic!("test binary: {error}")),
    )
    .args(["--exact", TEST, "--nocapture", "--test-threads=1"])
    .env(CHILD, TEST)
    .env("TERM", "xterm")
    .stdin(Stdio::null())
    .stdout(Stdio::piped());
    for name in [
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "NO_COLOR",
        "COLORTERM",
        "COLUMNS",
        DEFAULT_COLUMN_BUFFER_ENV,
    ] {
        command = command.env_remove(name);
    }
    let mut process = command
        .spawn(pts)
        .unwrap_or_else(|error| panic!("start the child: {error}"));
    let (sender, received) = mpsc::channel();
    thread::spawn(move || {
        let mut seen = Vec::new();
        let mut buffer = [0; 4096];
        while let Ok(read @ 1..) = (&pty).read(&mut buffer) {
            seen.extend_from_slice(&buffer[..read]);
        }
        let _ = sender.send(seen);
    });
    let status = process
        .wait()
        .unwrap_or_else(|error| panic!("wait for the child: {error}"));
    assert!(status.success(), "the child failed: {status}");
    let seen = received
        .recv_timeout(Duration::from_secs(20))
        .unwrap_or_else(|error| panic!("read the terminal: {error}"));
    let stderr = String::from_utf8_lossy(&seen);
    assert!(
        stderr.contains("\u{1b}["),
        "help should be coloured: {stderr:?}"
    );
    let help = plain(&stderr);
    let lines = help
        .split('\n')
        .map(|line| line.trim_end_matches('\r'))
        .collect::<Vec<_>>();
    assert!(lines.iter().any(|line| line.contains("Usage")), "{help}");
    assert!(lines.iter().all(|line| width(line) <= 48), "{help}");
}

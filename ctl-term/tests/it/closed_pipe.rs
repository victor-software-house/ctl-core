//! A reader that closes stdout early, through a real child process piped to
//! `head -1`. The test binary runs itself as the writer: the parent starts it
//! again with a switch variable, and the writer test only acts when that
//! variable is set.
#![cfg(unix)]

use std::process::{Command, Stdio};

use ctl_term::{ColorMode, Console, Error, Stream};

use crate::common::{Rows, line, red};

/// The variable that turns `writer_child` from a no-op into the writer.
const WRITER_SWITCH: &str = "CTL_TERM_TEST_WRITER";

/// The variables an agent shell sets and the writer must not inherit.
const COLOUR_VARIABLES: [&str; 5] = [
    "CLICOLOR",
    "CLICOLOR_FORCE",
    "FORCE_COLOR",
    "NO_COLOR",
    "COLORTERM",
];

const ROWS: usize = 10_000;

/// The writer: prints 10,000 rows and exits 0 when the console reports a
/// closed pipe, 3 when every row went through, and 4 on any other error.
#[test]
fn writer_child() {
    if std::env::var_os(WRITER_SWITCH).is_none() {
        return;
    }
    let rows = Rows(
        (0..ROWS)
            .map(|row| line(&[(format!("row {row:05} of a long list").as_str(), red())]))
            .collect(),
    );
    let console = Console::from_process(Stream::Stdout, ColorMode::Auto);
    let code = match console.print(&rows) {
        Ok(()) => 3,
        Err(Error::BrokenPipe(_)) => 0,
        Err(_) => 4,
    };
    std::process::exit(code);
}

#[test]
fn a_reader_that_stops_early_ends_the_writer_with_broken_pipe() {
    let mut head = Command::new("head")
        .arg("-1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start head");
    let pipe = head.stdin.take().expect("head stdin");

    let mut writer = Command::new(std::env::current_exe().expect("path of the test binary"));
    writer
        .args([
            "--exact",
            "closed_pipe::writer_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(WRITER_SWITCH, "1")
        .stdout(Stdio::from(pipe))
        .stderr(Stdio::piped());
    for name in COLOUR_VARIABLES {
        writer.env_remove(name);
    }
    let output = writer
        .spawn()
        .expect("start the writer")
        .wait_with_output()
        .expect("wait for the writer");
    let head_output = head.wait_with_output().expect("wait for head");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the writer saw no BrokenPipe; stderr: {stderr}"
    );
    assert!(stderr.is_empty(), "stderr: {stderr}");
    let kept = String::from_utf8_lossy(&head_output.stdout);
    assert_eq!(kept.lines().count(), 1, "head kept: {kept}");
}

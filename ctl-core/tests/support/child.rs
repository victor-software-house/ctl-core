//! Run one test of the calling test binary again as a child process, to read
//! what the code under test wrote to stdout and stderr.

use std::env;
use std::io::{self, Write as _};
use std::process::{Command, Output};

use ctl_core::prelude::*;

/// The command that reruns `test` of this binary as a child with `vars` set,
/// or `None` when this process is that child and should run the case.
/// `variable` marks the child.
///
/// The child writes to pipes, so `COLUMNS`, the colour variables, and
/// ctl-core's layout variables would change its output; it runs without them,
/// at the fallback width.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
pub(crate) fn command(variable: &str, test: &str, vars: &[(&str, &str)]) -> Option<Command> {
    if env::var_os(variable).as_deref() == Some(test.as_ref()) {
        return None;
    }
    let mut child = Command::new(env::current_exe().expect("test binary"));
    child
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(variable, test)
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("FORCE_COLOR")
        .env_remove("NO_COLOR")
        .env_remove("COLORTERM")
        .env_remove("COLUMNS")
        .env_remove(DEFAULT_COLUMN_BUFFER_ENV)
        .env_remove(DEFAULT_RECORD_STYLE_ENV)
        .env_remove(DEFAULT_LIST_STYLE_ENV)
        .env_remove(DEFAULT_ROW_SEPARATION_ENV)
        .env_remove(DEFAULT_JSON_LAYOUT_ENV)
        .envs(vars.iter().copied());
    Some(child)
}

/// Run `test` in a child with `vars` set and return its output, or `None`
/// when this process is that child and should run the case. `variable`
/// marks the child. The child's own assertions fail the parent.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
pub(crate) fn in_child(variable: &str, test: &str, vars: &[(&str, &str)]) -> Option<Output> {
    let output = command(variable, test, vars)?
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

/// The line the child prints to stdout before the case runs.
pub(crate) const CASE_START: &str = "--- case start ---\n";
/// The line the child prints to stdout after the case ran.
const CASE_END: &str = "--- case end ---\n";

/// Run the case between two marker lines on stdout, so [`case_stdout`] can
/// tell what the case wrote from what the test harness prints around it.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
pub(crate) fn case<T>(run: impl FnOnce() -> T) -> T {
    let mark = |line: &str| {
        let mut stdout = io::stdout().lock();
        stdout.write_all(line.as_bytes()).expect("write a marker");
        stdout.flush().expect("flush a marker");
    };
    mark(CASE_START);
    let value = run();
    mark(CASE_END);
    value
}

/// What the case wrote to stdout, between the markers [`case`] prints.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
pub(crate) fn case_stdout(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (_, rest) = stdout
        .split_once(CASE_START)
        .expect("the case start marker");
    let (written, _) = rest.split_once(CASE_END).expect("the case end marker");
    written.to_owned()
}

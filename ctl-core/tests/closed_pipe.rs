//! A CLI built on `App` whose reader closes stdout early, through a real
//! child process piped to `head -1`.
//!
//! The target has no libtest harness: the harness prints its own banner to
//! stdout, and `head -1` quits after the first line, so the next harness
//! write would fail before the code under test ran. With no harness, only
//! `App` writes to the pipe.
//!
//! The binary runs as the parent, then once as the writer for each closed-pipe
//! policy. Started plainly it is the parent: for each policy it starts
//! `head -1`, then starts itself with the switch variable naming the policy
//! and its stdout on `head`'s stdin. With the switch set it is the writer: it
//! runs `App` on a command that prints 10,000 rows and exits with `App`'s
//! code. Arguments are ignored, so `cargo test <filter>` still runs this
//! binary.

#[cfg(unix)]
mod unix {
    use std::io::{self, Write};
    use std::path::PathBuf;
    use std::process::{Child, Command, ExitCode, Stdio};

    use ctl_core::prelude::*;
    use serde::Serialize;

    /// The variable that makes the binary the writer, set to the policy.
    const WRITER_SWITCH: &str = "CTL_CORE_TEST_PIPE_WRITER";

    /// The variables an agent shell sets and the writer must not inherit.
    const COLOUR_VARIABLES: [&str; 5] = [
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "NO_COLOR",
        "COLORTERM",
    ];

    const ROWS: usize = 10_000;

    #[derive(Parser)]
    #[command(name = "rows")]
    struct Rows {}

    #[derive(Serialize)]
    struct List {
        rows: Vec<String>,
    }

    impl Present for List {
        fn present(&self) -> Document {
            Document::new().verbatim(self.rows.join("\n"))
        }
    }

    pub(super) fn run() -> ExitCode {
        if let Some(policy) = std::env::var_os(WRITER_SWITCH) {
            return write_rows(if policy == "fail" {
                BrokenPipe::Fail
            } else {
                BrokenPipe::Succeed
            });
        }
        let checked = check("succeed", |code| code == Some(0))
            .and_then(|()| check("fail", |code| code.is_some_and(|code| code != 0)));
        match checked {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                writeln!(io::stderr(), "closed_pipe: {message}").ok();
                ExitCode::FAILURE
            }
        }
    }

    fn write_rows(policy: BrokenPipe) -> ExitCode {
        App::<Rows>::new("rows")
            .broken_pipe(policy)
            .run_from(["rows"], |_| {
                Ok(List {
                    rows: (0..ROWS)
                        .map(|row| format!("row {row:05} of a long list"))
                        .collect(),
                })
            })
    }

    /// Starts the writer with `stdout` as its standard output.
    ///
    /// The `Command` keeps its copy of `stdout`, the write end of `head`'s
    /// stdin, until it is dropped. It is dropped when this returns, so a
    /// writer that dies before writing a line leaves `head` at end of file
    /// and the test fails instead of hanging.
    fn spawn_writer(path: PathBuf, policy: &str, stdout: Stdio) -> io::Result<Child> {
        let mut writer = Command::new(path);
        writer
            .env(WRITER_SWITCH, policy)
            .stdout(stdout)
            .stderr(Stdio::piped());
        for name in COLOUR_VARIABLES {
            writer.env_remove(name);
        }
        writer.spawn()
    }

    /// Runs the writer under `policy` into `head -1`, and checks that its exit
    /// code passes `expected`, that it wrote nothing to stderr, and that
    /// `head` kept one line.
    fn check(policy: &str, expected: impl Fn(Option<i32>) -> bool) -> Result<(), String> {
        let mut head = Command::new("head")
            .arg("-1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| format!("could not start head: {error}"))?;
        let pipe = head
            .stdin
            .take()
            .ok_or_else(|| "head has no stdin".to_owned())?;

        let path = std::env::current_exe()
            .map_err(|error| format!("could not find the test binary: {error}"))?;
        let child = spawn_writer(path, policy, Stdio::from(pipe))
            .map_err(|error| format!("could not start the writer: {error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("could not wait for the writer: {error}"))?;
        let head_output = head
            .wait_with_output()
            .map_err(|error| format!("could not wait for head: {error}"))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !expected(output.status.code()) {
            return Err(format!(
                "under {policy}, the writer exited with {:?}; stderr: {stderr}",
                output.status
            ));
        }
        if !stderr.is_empty() {
            return Err(format!(
                "under {policy}, the writer wrote to stderr: {stderr}"
            ));
        }
        let kept = String::from_utf8_lossy(&head_output.stdout);
        if kept.lines().count() != 1 {
            return Err(format!("head kept {} lines: {kept}", kept.lines().count()));
        }
        Ok(())
    }
}

/// Relies on Rust's default of ignoring SIGPIPE, so a closed pipe reaches the
/// writer as an I/O error; `-Zon-broken-pipe=kill` or a SIGPIPE reset would
/// kill it by the signal instead, and the parent would report a failure.
#[cfg(unix)]
fn main() -> std::process::ExitCode {
    unix::run()
}

#[cfg(not(unix))]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::SUCCESS
}

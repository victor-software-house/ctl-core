//! A CLI built on `App` whose reader closes its output early, through a real
//! child process and a pipe that the reader closes after one line.
//!
//! The target has no libtest harness: the harness prints its own banner to
//! stdout, and the reader closes the pipe after the first line, so the next
//! harness write would fail before the code under test ran. With no harness,
//! only `App` writes to the pipe.
//!
//! Started plainly, the binary is the parent and the reader. For each case it
//! opens a pipe, then starts itself as the writer, with the case and the
//! closed-pipe policy in the switch variables and the case's stream on the
//! pipe's write end. The parent reads the first line, then closes the read
//! end. The writer runs `App` and exits with `App`'s code. Each case writes
//! more than the pipe buffers and the parent reads together, so the write
//! fails once the parent has closed the pipe. The parent reads the line
//! itself, not through `head`: how much a system `head` reads before it quits
//! differs between implementations, and one that reads a whole pipe buffer
//! lets the writer finish. Arguments are ignored, so `cargo test <filter>`
//! still runs this binary.

use std::process::ExitCode;

#[cfg(unix)]
mod unix {
    use std::env;
    use std::io::{self, BufRead, BufReader, Write};
    use std::path::PathBuf;
    use std::process::{Child, Command, ExitCode, Stdio};

    use anyhow::anyhow;
    use ctl_core::prelude::*;
    use serde::Serialize;

    /// The variable that makes the binary the writer, set to the case.
    const CASE_SWITCH: &str = "CTL_CORE_TEST_PIPE_CASE";

    /// The variable holding the writer's closed-pipe policy.
    const POLICY_SWITCH: &str = "CTL_CORE_TEST_PIPE_POLICY";

    /// The variables an agent shell sets and the writer must not inherit.
    const COLOUR_VARIABLES: [&str; 5] = [
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "NO_COLOR",
        "COLORTERM",
    ];

    /// How much the parent reads from the pipe at a time. The rows case's
    /// first line is 11 bytes, so the parent reads at most this much before
    /// it closes the pipe.
    const READ_BUFFER: usize = 4096;

    /// Words in each case's output, 11 bytes each with their separator:
    /// 110 KB. That is over the 64 KiB a Linux pipe buffers plus the
    /// [`READ_BUFFER`] the parent reads, 68 KiB, and under Linux's 128 KiB
    /// limit on one argument.
    const WORDS: usize = 10_000;

    #[derive(Parser)]
    #[command(name = "rows", version)]
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

    /// What the writer does, and so which stream the parent reads.
    #[derive(Clone, Copy)]
    enum Case {
        /// A command that succeeds and prints rows on stdout.
        Rows,
        /// A command that fails with a long error on stderr.
        Error,
        /// A parse error, with a long unexpected argument, on stderr.
        Parse,
    }

    impl Case {
        const ALL: [Self; 3] = [Self::Rows, Self::Error, Self::Parse];

        const fn name(self) -> &'static str {
            match self {
                Self::Rows => "rows",
                Self::Error => "error",
                Self::Parse => "parse",
            }
        }

        fn named(name: &str) -> Option<Self> {
            Self::ALL.into_iter().find(|case| case.name() == name)
        }

        /// The exit code the writer must end with under `policy`.
        const fn expected(self, policy: BrokenPipe) -> i32 {
            match (self, policy) {
                (Self::Rows, BrokenPipe::Ignore) => 0,
                (Self::Rows | Self::Error, BrokenPipe::Fail)
                | (Self::Error, BrokenPipe::Ignore) => 1,
                (Self::Parse, BrokenPipe::Ignore | BrokenPipe::Fail) => 2,
            }
        }
    }

    const fn policy_name(policy: BrokenPipe) -> &'static str {
        match policy {
            BrokenPipe::Ignore => "ignore",
            BrokenPipe::Fail => "fail",
        }
    }

    fn words() -> impl Iterator<Item = String> {
        (0..WORDS).map(|word| format!("word-{word:05}"))
    }

    pub(super) fn run() -> ExitCode {
        if let Some(case) = env::var(CASE_SWITCH).ok().as_deref().and_then(Case::named) {
            let policy = if env::var(POLICY_SWITCH).as_deref() == Ok("fail") {
                BrokenPipe::Fail
            } else {
                BrokenPipe::Ignore
            };
            return write(case, policy);
        }
        let checked = Case::ALL.into_iter().try_for_each(|case| {
            [BrokenPipe::Ignore, BrokenPipe::Fail]
                .into_iter()
                .try_for_each(|policy| check(case, policy))
        });
        match checked {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                writeln!(io::stderr(), "closed_pipe: {message}").ok();
                ExitCode::FAILURE
            }
        }
    }

    fn write(case: Case, policy: BrokenPipe) -> ExitCode {
        let app = App::<Rows>::new("rows").broken_pipe(policy);
        match case {
            Case::Rows => app.run_from(["rows"], |_| {
                Ok(List {
                    rows: words().collect(),
                })
            }),
            Case::Error => app.run_from(["rows"], |_| -> anyhow::Result<List> {
                Err(anyhow!(words().collect::<Vec<_>>().join(" ")))
            }),
            Case::Parse => {
                let unexpected = words().collect::<Vec<_>>().join(" ");
                app.run_from(["rows".to_owned(), unexpected], |_| {
                    Ok(List { rows: Vec::new() })
                })
            }
        }
    }

    /// Starts the writer for `case` under `policy`, with `pipe` as the stream
    /// the case writes to and the other stream captured.
    ///
    /// The `Command` keeps its copy of `pipe`, the pipe's write end, until it
    /// is dropped. It is dropped when this returns, so a writer that dies
    /// before writing a line leaves the parent's read at end of file and the
    /// test fails instead of hanging.
    fn spawn_writer(
        path: PathBuf,
        case: Case,
        policy: BrokenPipe,
        pipe: Stdio,
    ) -> io::Result<Child> {
        let mut writer = Command::new(path);
        writer
            .env(CASE_SWITCH, case.name())
            .env(POLICY_SWITCH, policy_name(policy));
        match case {
            Case::Rows => writer.stdout(pipe).stderr(Stdio::piped()),
            Case::Error | Case::Parse => writer.stdout(Stdio::piped()).stderr(pipe),
        };
        for name in COLOUR_VARIABLES {
            writer.env_remove(name);
        }
        writer.spawn()
    }

    /// Runs the writer for `case` under `policy` into a pipe that this reads
    /// one line from and then closes, and checks the writer's exit code, that
    /// the stream the parent did not read stayed empty, and that the parent
    /// read a whole line.
    fn check(case: Case, policy: BrokenPipe) -> Result<(), String> {
        let label = format!("{} under {}", case.name(), policy_name(policy));
        let (reader, pipe) =
            io::pipe().map_err(|error| format!("could not open a pipe: {error}"))?;

        let path = env::current_exe()
            .map_err(|error| format!("could not find the test binary: {error}"))?;
        let child = spawn_writer(path, case, policy, Stdio::from(pipe))
            .map_err(|error| format!("{label}: could not start the writer: {error}"))?;
        let mut reader = BufReader::with_capacity(READ_BUFFER, reader);
        let mut first = String::new();
        reader
            .read_line(&mut first)
            .map_err(|error| format!("{label}: could not read the first line: {error}"))?;
        drop(reader);
        let output = child
            .wait_with_output()
            .map_err(|error| format!("{label}: could not wait for the writer: {error}"))?;

        let other = match case {
            Case::Rows => &output.stderr,
            Case::Error | Case::Parse => &output.stdout,
        };
        let other = String::from_utf8_lossy(other);
        let expected = case.expected(policy);
        if output.status.code() != Some(expected) {
            return Err(format!(
                "{label}: the writer exited with {:?}, not {expected}; other stream: {other}",
                output.status
            ));
        }
        if !other.is_empty() {
            return Err(format!(
                "{label}: the writer wrote to its other stream: {other}"
            ));
        }
        if !first.ends_with('\n') {
            return Err(format!(
                "{label}: the parent read no whole line, only {} bytes",
                first.len()
            ));
        }
        Ok(())
    }
}

/// Relies on Rust's default of ignoring SIGPIPE, so a closed pipe reaches the
/// writer as an I/O error; `-Zon-broken-pipe=kill` or a SIGPIPE reset would
/// kill it by the signal instead, and the parent would report a failure.
#[cfg(unix)]
fn main() -> ExitCode {
    unix::run()
}

#[cfg(not(unix))]
fn main() -> ExitCode {
    ExitCode::SUCCESS
}

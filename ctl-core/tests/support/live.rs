//! A child process a test talks to line by line: it reads each line the
//! child writes to stdout under a deadline and writes lines to its stdin.
//!
//! [`child::in_child`] returns the child's output once it has ended. A test of
//! a stream needs the child while it runs, so it can read one record, answer,
//! or close the pipe. The child cannot be an ordinary libtest case that
//! returns: once the reader closed stdout, the harness would panic writing its
//! result line. [`serve`] runs the case and ends the process itself.

use std::io::{self, BufRead as _, BufReader, Read as _, Write as _};
use std::process::{self, Child, ChildStdin, ExitCode, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::child::{self, CASE_START};

/// How long a read, or the wait for the child to end, may take before the
/// test fails. A run that collects its records before it writes any deadlocks
/// against a test that answers each line, and ends here.
const DEADLINE: Duration = Duration::from_secs(10);

/// A running child, as its parent sees it.
pub(crate) struct Live {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

/// How the child ended: its status and what it wrote to stderr.
pub(crate) struct Ended {
    pub(crate) status: ExitStatus,
    pub(crate) stderr: String,
}

/// Start `test` of this binary as a child with `vars` set, and skip what the
/// harness writes before the case. Returns `None` when this process is that
/// child, which then calls [`serve`].
pub(crate) fn spawn(variable: &str, test: &str, vars: &[(&str, &str)]) -> Option<Live> {
    let mut command = child::command(variable, test, vars)?;
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("could not start the child: {error}"));
    let stdout = child.stdout.take();
    let stdin = child.stdin.take();
    let (send, lines) = mpsc::channel();
    thread::spawn(move || {
        let Some(stdout) = stdout else { return };
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if send.send(line).is_err() {
                return;
            }
        }
    });
    let mut live = Live {
        child,
        stdin,
        lines,
    };
    while !live.read_line().contains(CASE_START.trim_end()) {}
    Some(live)
}

impl Live {
    /// The next line the child wrote to stdout, without its line end.
    pub(crate) fn read_line(&mut self) -> String {
        match self.lines.recv_timeout(DEADLINE) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => {
                self.child.kill().ok();
                panic!("the child wrote no line within {DEADLINE:?}");
            }
            Err(RecvTimeoutError::Disconnected) => {
                panic!("the child closed stdout before it wrote the line")
            }
        }
    }

    /// Write one line to the child's stdin.
    pub(crate) fn write_line(&mut self, text: &str) {
        let stdin = self
            .stdin
            .as_mut()
            .unwrap_or_else(|| panic!("the child's stdin is closed"));
        writeln!(stdin, "{text}")
            .and_then(|()| stdin.flush())
            .unwrap_or_else(|error| panic!("could not write to the child: {error}"));
    }

    /// Close the pipes the parent holds and wait for the child to end. The
    /// child sees its stdin at end of file, and a write to its stdout fails
    /// once the reader thread notices that nothing reads the lines.
    pub(crate) fn finish(self) -> Ended {
        let Self {
            mut child,
            stdin,
            lines,
        } = self;
        drop(stdin);
        drop(lines);
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if start.elapsed() < DEADLINE => thread::sleep(Duration::from_millis(10)),
                Ok(None) => {
                    child.kill().ok();
                    panic!("the child did not end within {DEADLINE:?}");
                }
                Err(error) => panic!("could not wait for the child: {error}"),
            }
        };
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            pipe.read_to_string(&mut stderr)
                .unwrap_or_else(|error| panic!("could not read the child's stderr: {error}"));
        }
        Ended { status, stderr }
    }
}

/// The child's side of [`spawn`]: mark where the case starts, run it, check
/// its exit code against `expected`, and end the process with status 0.
///
/// A failed assertion ends the child with the harness's failure status, which
/// the parent reads. The process ends here and not in the harness, whose
/// result line fails to write once the parent closed stdout.
pub(crate) fn serve(expected: ExitCode, run: impl FnOnce() -> ExitCode) -> ! {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(CASE_START.as_bytes())
        .and_then(|()| stdout.flush())
        .unwrap_or_else(|error| panic!("could not write the start marker: {error}"));
    drop(stdout);
    assert_eq!(run(), expected);
    process::exit(0)
}

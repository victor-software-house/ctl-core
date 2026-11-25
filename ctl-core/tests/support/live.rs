//! A child process a test talks to line by line: it reads each line the
//! child writes to stdout under a deadline and writes lines to its stdin.
//!
//! [`child::in_child`] returns the child's output once it has ended. A test of
//! a stream needs the child while it runs, so it can read one record, answer,
//! or close the pipe. The child cannot be an ordinary libtest case that
//! returns: once the reader closed stdout, the harness would panic writing its
//! result line. [`serve`] runs the case and ends the process itself.

use std::io::{self, BufRead as _, BufReader, Read as _, Write as _};
use std::process::{self, Child, ChildStdin, ChildStdout, ExitCode, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::child::{self, CASE_START};

/// How long a read, or the wait for the child to end, may take before the
/// test fails. A run that collects its records before it writes any deadlocks
/// against a test that answers each line, and ends here.
const DEADLINE: Duration = Duration::from_secs(10);

/// A running child, as its parent sees it.
///
/// A test that fails while the child runs ends the child when the `Live`
/// drops, so no process outlives its test.
pub(crate) struct Live {
    child: Child,
    stdin: Option<ChildStdin>,
    /// Asks the reader of stdout for one line. The reader reads only when
    /// asked, so dropping this closes the child's stdout while the reader
    /// idles, at a moment the test picks.
    asks: Option<Sender<()>>,
    lines: Receiver<Option<String>>,
    reader: Option<JoinHandle<()>>,
    stderr: Option<JoinHandle<String>>,
}

/// How the child ended: its status and what it wrote to stderr.
pub(crate) struct Ended {
    pub(crate) status: ExitStatus,
    pub(crate) stderr: String,
}

/// The reader of the child's stdout: it reads one line each time it is asked,
/// and answers `None` when the pipe ends.
fn read_lines(stdout: ChildStdout, asks: &Receiver<()>, lines: &Sender<Option<String>>) {
    let mut stdout = BufReader::new(stdout);
    while asks.recv().is_ok() {
        let mut line = String::new();
        let answer = match stdout.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim_end_matches(['\r', '\n']).to_owned()),
        };
        if lines.send(answer).is_err() {
            return;
        }
    }
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
    let stderr = child.stderr.take();
    let (asks, asked) = mpsc::channel();
    let (answer, lines) = mpsc::channel();
    let reader = thread::spawn(move || {
        if let Some(stdout) = stdout {
            read_lines(stdout, &asked, &answer);
        }
    });
    let stderr = thread::spawn(move || {
        let mut written = String::new();
        if let Some(mut pipe) = stderr {
            pipe.read_to_string(&mut written).ok();
        }
        written
    });
    let mut live = Live {
        child,
        stdin,
        asks: Some(asks),
        lines,
        reader: Some(reader),
        stderr: Some(stderr),
    };
    while !live.read_line().contains(CASE_START.trim_end()) {}
    Some(live)
}

impl Live {
    /// Kill the child and wait for it, so a failing test leaves no process.
    fn kill(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }

    /// The next line the child wrote to stdout, without its line end.
    pub(crate) fn read_line(&mut self) -> String {
        let asked = self.asks.as_ref().is_some_and(|asks| asks.send(()).is_ok());
        assert!(asked, "the parent closed the child's stdout");
        match self.lines.recv_timeout(DEADLINE) {
            Ok(Some(line)) => line,
            Ok(None) | Err(RecvTimeoutError::Disconnected) => {
                self.kill();
                panic!("the child closed stdout before it wrote the line");
            }
            Err(RecvTimeoutError::Timeout) => {
                self.kill();
                panic!("the child wrote no line within {DEADLINE:?}");
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

    /// Close the read end of the child's stdout. The child's next write to it
    /// fails, and no other read of it is possible.
    pub(crate) fn close_stdout(&mut self) {
        self.asks = None;
        if let Some(reader) = self.reader.take() {
            reader
                .join()
                .unwrap_or_else(|_| panic!("the reader of stdout panicked"));
        }
    }

    /// Close the pipes the parent holds and wait for the child to end. The
    /// child sees its stdin at end of file, and a write to its stdout fails.
    pub(crate) fn finish(mut self) -> Ended {
        self.stdin = None;
        self.close_stdout();
        let start = Instant::now();
        let status = loop {
            match self.child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if start.elapsed() < DEADLINE => thread::sleep(Duration::from_millis(10)),
                Ok(None) => {
                    self.kill();
                    panic!("the child did not end within {DEADLINE:?}");
                }
                Err(error) => panic!("could not wait for the child: {error}"),
            }
        };
        let stderr = self
            .stderr
            .take()
            .map(JoinHandle::join)
            .and_then(Result::ok)
            .unwrap_or_default();
        Ended { status, stderr }
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            self.kill();
        }
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

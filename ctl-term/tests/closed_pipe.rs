//! A reader that closes stdout early, through a real child process and a
//! pipe that the reader closes after one line.
//!
//! The target has no libtest harness: the harness prints its own banner to
//! stdout, and the reader closes the pipe after the first line, so the next
//! harness write would fail before the code under test ran. With no harness,
//! only [`Console`] writes to the pipe.
//!
//! The binary runs twice. Started plainly it is the parent and the reader: it
//! opens a pipe, starts itself with the switch variable set and its stdout on
//! the pipe's write end, reads the first line, then closes the read end. With
//! the switch set it is the writer: it prints 10,000 rows and exits 0 when the
//! console reports a closed pipe, 3 when every row went through, and 4 on any
//! other error. The parent reads the line itself, not through `head`: how
//! much a system `head` reads before it quits differs between
//! implementations, and one that reads a whole pipe buffer lets the writer
//! finish. Arguments are ignored, so `cargo test <filter>` still runs this
//! binary.

use std::process::ExitCode;

#[cfg(unix)]
mod unix {
    use std::env;
    use std::io::{self, BufRead, BufReader, Write};
    use std::path::PathBuf;
    use std::process::{Child, Command, ExitCode, Stdio};

    use ctl_term::{ColorMode, Console, DetectOptions, Error, Stream};
    use ctl_text::{Context, Line, Measurement, Renderable, Span, Style};

    /// The variable that makes the binary the writer.
    const WRITER_SWITCH: &str = "CTL_TERM_TEST_WRITER";

    /// The variables an agent shell sets and the writer must not inherit.
    const COLOUR_VARIABLES: [&str; 5] = [
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "NO_COLOR",
        "COLORTERM",
    ];

    /// How much the parent reads from the pipe at a time. The first row is
    /// 25 bytes, so the parent reads at most this much before it closes the
    /// pipe.
    const READ_BUFFER: usize = 4096;

    /// Rows the writer prints, 25 bytes each with their line break: 250 KB,
    /// over the 64 KiB a Linux pipe buffers plus the [`READ_BUFFER`] the
    /// parent reads, 68 KiB.
    const ROWS: usize = 10_000;

    /// Lines handed to the console as they are.
    #[derive(Debug)]
    struct Rows(Vec<Line>);

    impl Renderable for Rows {
        fn measure(&self, _: &Context) -> Measurement {
            Measurement::exact(self.0.iter().map(Line::width).max().unwrap_or(0))
        }

        fn render(&self, _: &Context) -> Vec<Line> {
            self.0.clone()
        }
    }

    pub(super) fn run() -> ExitCode {
        if env::var_os(WRITER_SWITCH).is_some() {
            return write_rows();
        }
        match start_both() {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                writeln!(io::stderr(), "closed_pipe: {message}").ok();
                ExitCode::FAILURE
            }
        }
    }

    fn write_rows() -> ExitCode {
        let rows = Rows(
            (0..ROWS)
                .map(|row| {
                    let mut line = Line::new();
                    line.push(Span::new(
                        format!("row {row:05} of a long list"),
                        Style::new(),
                    ));
                    line
                })
                .collect(),
        );
        let console = Console::from_process(Stream::Stdout, ColorMode::Auto, DetectOptions::new());
        match console.print(&rows) {
            Ok(()) => ExitCode::from(3),
            Err(Error::BrokenPipe(_)) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(4),
        }
    }

    /// Starts the writer with `stdout` as its standard output.
    ///
    /// The `Command` keeps its copy of `stdout`, the pipe's write end, until
    /// it is dropped. It is dropped when this returns, so a writer that dies
    /// before writing a line leaves the parent's read at end of file and the
    /// test fails instead of hanging.
    fn spawn_writer(path: PathBuf, stdout: Stdio) -> io::Result<Child> {
        let mut writer = Command::new(path);
        writer
            .env(WRITER_SWITCH, "1")
            .stdout(stdout)
            .stderr(Stdio::piped());
        for name in COLOUR_VARIABLES {
            writer.env_remove(name);
        }
        writer.spawn()
    }

    /// Starts the writer on a pipe, reads one line from it and closes it,
    /// then checks that the writer saw the closed pipe, wrote nothing to
    /// stderr, and that the parent read a whole line.
    fn start_both() -> Result<(), String> {
        let (reader, pipe) =
            io::pipe().map_err(|error| format!("could not open a pipe: {error}"))?;

        let path = env::current_exe()
            .map_err(|error| format!("could not find the test binary: {error}"))?;
        let child = spawn_writer(path, Stdio::from(pipe))
            .map_err(|error| format!("could not start the writer: {error}"))?;
        let mut reader = BufReader::with_capacity(READ_BUFFER, reader);
        let mut first = String::new();
        reader
            .read_line(&mut first)
            .map_err(|error| format!("could not read the first line: {error}"))?;
        drop(reader);
        let output = child
            .wait_with_output()
            .map_err(|error| format!("could not wait for the writer: {error}"))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.code() != Some(0) {
            return Err(format!(
                "the writer did not see a closed pipe: {:?}; stderr: {stderr}",
                output.status
            ));
        }
        if !stderr.is_empty() {
            return Err(format!("the writer wrote to stderr: {stderr}"));
        }
        if !first.ends_with('\n') {
            return Err(format!(
                "the parent read no whole line, only {} bytes",
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
    use std::io::{self, Write};

    writeln!(
        io::stderr(),
        "closed_pipe: skipped, ctl-term supports and tests Unix only"
    )
    .ok();
    ExitCode::SUCCESS
}

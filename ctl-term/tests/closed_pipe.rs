//! A reader that closes stdout early, through a real child process piped to
//! `head -1`.
//!
//! The target has no libtest harness: the harness prints its own banner to
//! stdout, and `head -1` quits after the first line, so the next harness
//! write would fail before the code under test ran. With no harness, only
//! [`Console`] writes to the pipe.
//!
//! The binary runs twice. Started plainly it is the parent: it starts
//! `head -1`, then starts itself with the switch variable set and its stdout
//! on `head`'s stdin. With the switch set it is the writer: it prints 10,000
//! rows and exits 0 when the console reports a closed pipe, 3 when every row
//! went through, and 4 on any other error. Arguments are ignored.

#[cfg(unix)]
mod unix {
    use std::io::{self, Write};
    use std::process::{Command, ExitCode, Stdio};

    use ctl_term::{ColorMode, Console, Error, Stream};
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
        if std::env::var_os(WRITER_SWITCH).is_some() {
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
        let console = Console::from_process(Stream::Stdout, ColorMode::Auto);
        match console.print(&rows) {
            Ok(()) => ExitCode::from(3),
            Err(Error::BrokenPipe(_)) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(4),
        }
    }

    fn start_both() -> Result<(), String> {
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
        let mut writer = Command::new(path);
        writer
            .env(WRITER_SWITCH, "1")
            .stdout(Stdio::from(pipe))
            .stderr(Stdio::piped());
        for name in COLOUR_VARIABLES {
            writer.env_remove(name);
        }
        let output = writer
            .spawn()
            .and_then(std::process::Child::wait_with_output)
            .map_err(|error| format!("could not run the writer: {error}"))?;
        let head_output = head
            .wait_with_output()
            .map_err(|error| format!("could not wait for head: {error}"))?;

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
        let kept = String::from_utf8_lossy(&head_output.stdout);
        if kept.lines().count() != 1 {
            return Err(format!("head kept {} lines: {kept}", kept.lines().count()));
        }
        Ok(())
    }
}

#[cfg(unix)]
fn main() -> std::process::ExitCode {
    unix::run()
}

#[cfg(not(unix))]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::SUCCESS
}

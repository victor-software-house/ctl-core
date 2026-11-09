//! A `NO_COLOR` that is not valid UTF-8 still turns colour off, through a real
//! child process.
//!
//! A test cannot put such a value in its own environment without `unsafe`,
//! which the workspace forbids, so the binary starts itself with the value set
//! on the child. It has no libtest harness, so the child is a plain `main`
//! that reports through its exit code.
//!
//! Started plainly it is the parent. With the expectation variable set it is
//! the child: it reads the process environment through
//! `Terminal::from_process` and exits 0 when the colour matches what the
//! variable says. `CLICOLOR_FORCE` is set on both children, so a `NO_COLOR`
//! that went unread would leave colour on. The control child, without
//! `NO_COLOR`, proves the harness would show that. Arguments are ignored, so
//! `cargo test <filter>` still runs this binary.

#[cfg(unix)]
mod unix {
    use std::ffi::OsStr;
    use std::io::{self, Write};
    use std::os::unix::ffi::OsStrExt;
    use std::process::{Command, ExitCode};

    use ctl_term::{ColorMode, DetectOptions, Stream, Terminal};

    /// The variable that makes the binary the child, holding `on` or `off`.
    const EXPECTATION: &str = "CTL_TERM_TEST_EXPECT_COLOR";

    /// The variables an agent shell sets and the child must not inherit.
    const COLOUR_VARIABLES: [&str; 5] = [
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "NO_COLOR",
        "COLORTERM",
    ];

    pub(super) fn run() -> ExitCode {
        match std::env::var_os(EXPECTATION)
            .as_deref()
            .and_then(OsStr::to_str)
        {
            Some("on") => child(true),
            Some("off") => child(false),
            _ => parent(),
        }
    }

    fn child(expect_color: bool) -> ExitCode {
        let terminal =
            Terminal::from_process(Stream::Stdout, ColorMode::Auto, DetectOptions::new());
        if terminal.color().is_some() == expect_color {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }

    /// Runs the child expecting `expectation`, with `no_color` as its
    /// `NO_COLOR` when given.
    fn run_child(expectation: &str, no_color: Option<&OsStr>) -> Result<(), String> {
        let path = std::env::current_exe()
            .map_err(|error| format!("could not find the test binary: {error}"))?;
        let mut command = Command::new(path);
        for name in COLOUR_VARIABLES {
            command.env_remove(name);
        }
        command
            .env(EXPECTATION, expectation)
            .env("CLICOLOR_FORCE", "1");
        if let Some(value) = no_color {
            command.env("NO_COLOR", value);
        }
        let status = command
            .status()
            .map_err(|error| format!("could not start the child: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "the child expecting colour {expectation} exited with {status}"
            ))
        }
    }

    fn parent() -> ExitCode {
        let invalid = OsStr::from_bytes(b"\xff");
        match run_child("on", None).and_then(|()| run_child("off", Some(invalid))) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                writeln!(io::stderr(), "non_utf8_no_color: {message}").ok();
                ExitCode::FAILURE
            }
        }
    }
}

#[cfg(unix)]
fn main() -> std::process::ExitCode {
    unix::run()
}

#[cfg(not(unix))]
fn main() -> std::process::ExitCode {
    use std::io::{self, Write};

    writeln!(
        io::stderr(),
        "non_utf8_no_color: skipped, ctl-term supports and tests Unix only"
    )
    .ok();
    std::process::ExitCode::SUCCESS
}

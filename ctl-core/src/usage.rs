//! Mise Usage spec from a clap [`Command`].
//!
//! A CLI declares `--usage-spec[=BIN]` as a hidden flag with
//! `require_equals`. A mise file task that runs the CLI carries
//! [`mount_directives`], which end with [`mount_line`]: a `#USAGE mount`
//! line, from which mise asks the CLI for its spec. A user then runs
//! `mise run toy status` with no `--`. The `--` in the mount line is only how
//! mise fetches the spec, as mise documents.
//!
//! [`App::mounted_as`](crate::App::mounted_as) answers the request for an
//! [`App`](crate::App), and [`take`] does the same for a binary without one.

use std::io::Write as _;
use std::process::ExitCode;

use ::usage::Spec;
use clap::Command;

use crate::formatdoc;

/// Render the Usage KDL spec of `command` as a mise task named `bin`.
///
/// Both the spec's `name` and `bin` are set to `bin`, so mise shows the task
/// name and not the real binary name.
#[must_use]
pub fn spec(mut command: Command, bin: &str) -> String {
    command.set_bin_name(bin);
    let mut spec = Spec::from(&command);
    spec.name = bin.to_string();
    spec.bin = bin.to_string();
    spec.to_string()
}

/// The `#USAGE mount` line a mise file task carries for the task named
/// `task`.
///
/// The line runs `mise run --quiet <task> -- --usage-spec=<task>`. That
/// requests the task's own spec.
#[must_use]
pub fn mount_line(task: &str) -> String {
    formatdoc! {r#"#USAGE mount "mise run --quiet {task} -- --usage-spec={task}""#}
}

/// The mise directives a file task that runs a CLI carries for the task named
/// `task`, ending with [`mount_line`].
///
/// - `raw_args` passes argv through untouched.
/// - `raw` connects the CLI to the caller's stdin and stdout. Without it mise
///   relays each output line itself. Whenever mise's own colour is on, as under
///   `CLICOLOR_FORCE=1` even into a pipe, it appends an SGR reset to every
///   line. Then `--format json` piped to `jq` would carry an ESC byte the CLI
///   never wrote.
///
/// A raw task also gets no line prefix, and mise runs raw tasks one at a
/// time.
#[must_use]
pub fn mount_directives(task: &str) -> String {
    formatdoc! {"
        #MISE raw=true
        #MISE raw_args=true
        {mount}",
        mount = mount_line(task),
    }
}

/// If the process argv asks for `--usage-spec[=BIN]`, print the spec of `C` to
/// stdout and return the exit code to end with.
///
/// `default_bin` names the task when the flag has no `=BIN`. The result is
/// [`ExitCode::SUCCESS`] when the write worked and [`ExitCode::FAILURE`]
/// otherwise, and `None` when argv has no such flag.
#[must_use]
pub fn take<C: clap::CommandFactory>(default_bin: &str) -> Option<ExitCode> {
    let bin = spec_bin(std::env::args().skip(1), default_bin)?;
    let written = write!(std::io::stdout().lock(), "{}", spec(C::command(), &bin));
    Some(if written.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Read `--usage-spec` or `--usage-spec=BIN` from `args`, which exclude the
/// program name.
///
/// Returns the task name the spec is for: `BIN`, or `default_bin` for a bare
/// flag or an empty `BIN`. Scanning stops at `--`, so an argument after it
/// never counts.
#[must_use]
pub fn spec_bin(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    default_bin: &str,
) -> Option<String> {
    for arg in args {
        let arg = arg.as_ref();
        if arg == "--" {
            break;
        }
        if arg == "--usage-spec" {
            return Some(default_bin.to_owned());
        }
        if let Some(bin) = arg.strip_prefix("--usage-spec=") {
            return Some(if bin.is_empty() {
                default_bin.to_owned()
            } else {
                bin.to_owned()
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{mount_directives, mount_line, spec, spec_bin};

    #[derive(Parser)]
    #[command(name = "toy")]
    struct Toy {
        #[command(subcommand)]
        command: ToyCommand,
    }

    #[derive(clap::Subcommand)]
    enum ToyCommand {
        Status,
        Check,
    }

    #[test]
    fn spec_bin_reads_equals_and_bare() {
        assert_eq!(spec_bin(["--usage-spec"], "qctl").as_deref(), Some("qctl"));
        assert_eq!(spec_bin(["--usage-spec=q"], "qctl").as_deref(), Some("q"));
        assert_eq!(spec_bin(["status"], "qctl"), None);
        assert_eq!(spec_bin(["status", "--", "--usage-spec"], "qctl"), None);
        assert_eq!(
            spec_bin(["--usage-spec=q", "--", "status"], "qctl").as_deref(),
            Some("q")
        );
    }

    #[test]
    fn spec_names_the_mounted_bin() {
        let text = spec(Toy::command(), "q");
        assert!(text.contains("name") && text.contains("status"), "{text}");
    }

    #[test]
    fn mount_line_is_the_mise_bootstrap() {
        assert_eq!(
            mount_line("q"),
            r#"#USAGE mount "mise run --quiet q -- --usage-spec=q""#
        );
    }

    #[test]
    fn mount_directives_end_with_the_mount_line() {
        let directives = mount_directives("q");
        assert_eq!(
            directives.lines().collect::<Vec<_>>(),
            [
                "#MISE raw=true",
                "#MISE raw_args=true",
                mount_line("q").as_str()
            ]
        );
    }
}

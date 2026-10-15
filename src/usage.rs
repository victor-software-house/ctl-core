//! Mise Usage spec from a clap [`Command`].
//!
//! One hidden flag, one mount line, one operator form. A consumer writes
//! `--usage-spec[=BIN]` (hidden, `require_equals`). The served mise task
//! carries [`mount_directives`], which ends with [`mount_line`]. Operators then
//! run `mise run q status` — no `--`.
//! The `--` in the mount is only the completion bootstrap, as mise documents.
//!
//! Lefthook calls the same task: `mise run q close-from-git`.

use std::process::ExitCode;

use ::usage::Spec;
use clap::Command;

use crate::formatdoc;

/// Render a Usage KDL spec for a mise-mounted task named `bin`.
#[must_use]
pub fn spec(mut command: Command, bin: &str) -> String {
    command.set_bin_name(bin);
    let mut spec = Spec::from(&command);
    spec.name = bin.to_string();
    spec.bin = bin.to_string();
    spec.to_string()
}

/// The `#USAGE mount` line a served mise file task carries.
#[must_use]
pub fn mount_line(task: &str) -> String {
    formatdoc! {r#"#USAGE mount "mise run --quiet {task} -- --usage-spec={task}""#}
}

/// The mise directives a served CLI-proxy task carries, ending with
/// [`mount_line`]. `raw_args` passes argv through untouched. `raw` connects
/// the CLI to the caller's stdin and stdout: without it mise relays each
/// output line itself and, whenever its own colour is on (as under
/// `CLICOLOR_FORCE=1`, even into a pipe), appends an SGR reset to every line,
/// so `--format json` piped to `jq` carries an ESC byte the CLI never wrote.
/// A raw task also gets no line prefix, and mise runs raw tasks one at a time.
#[must_use]
pub fn mount_directives(task: &str) -> String {
    formatdoc! {"
        #MISE raw=true
        #MISE raw_args=true
        {mount}",
        mount = mount_line(task),
    }
}

/// If argv contains `--usage-spec[=BIN]`, print the spec for `C` and return
/// [`ExitCode::SUCCESS`].
#[must_use]
pub fn take<C: clap::CommandFactory>(default_bin: &str) -> Option<ExitCode> {
    let bin = spec_bin(std::env::args().skip(1), default_bin)?;
    print!("{}", spec(C::command(), &bin));
    Some(ExitCode::SUCCESS)
}

/// Parse `--usage-spec` / `--usage-spec=BIN` from argv (after the program
/// name).
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

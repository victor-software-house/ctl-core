//! Parser defaults. `-h` / `--help` and `-V` / `--version` stay on.

#[cfg(any(feature = "view", feature = "help"))]
use std::ffi::OsString;

use clap::{ColorChoice, Command};

#[cfg(any(feature = "view", feature = "help"))]
use crate::{ColorMode, OutputFormat};

/// Output policy recovered with the authoritative Clap graph before a full
/// parse succeeds.
#[cfg(any(feature = "view", feature = "help"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ParsedOutput {
    pub(crate) format: OutputFormat,
    pub(crate) color: ColorMode,
}

/// Best-effort output policy for help and parse errors.
///
/// Clap still owns option placement, attached short values, domain option
/// values, global propagation, and `--` semantics.
#[cfg(any(feature = "view", feature = "help"))]
pub(crate) fn parsed_output<C: clap::CommandFactory>(raw: &[OsString]) -> ParsedOutput {
    let command = C::command()
        .arg_required_else_help(false)
        .args_override_self(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .ignore_errors(true)
        .color(ColorChoice::Never);
    let Ok(matches) = command.try_get_matches_from(raw) else {
        return ParsedOutput {
            format: OutputFormat::Pretty,
            color: ColorMode::Auto,
        };
    };
    let format = matches
        .try_get_one::<OutputFormat>("format")
        .ok()
        .flatten()
        .copied()
        .unwrap_or_default();
    let color = matches
        .try_get_one::<ColorMode>("color")
        .ok()
        .flatten()
        .copied()
        .unwrap_or_default();
    let no_color = matches
        .try_get_one::<bool>("no_color")
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false);
    ParsedOutput {
        format,
        color: crate::flags::resolve_color(color, no_color),
    }
}

/// Apply the *ctl parser contract to a clap command.
///
/// `-h/--help` and `-V/--version` stay on. `disable_help_flag` is forbidden.
/// Repeated flags last-win (`args_override_self`);
/// [`crate::flags::chassis_warnings`] still reports the clash.
#[must_use]
pub fn apply_defaults(command: Command) -> Command {
    assert_help_enabled(&command);
    command
        .arg_required_else_help(true)
        .args_override_self(true)
        .color(ColorChoice::Auto)
        .disable_help_flag(false)
        .disable_version_flag(false)
}

/// clap's recommended `Command::debug_assert` plus the help-flag contract.
///
/// # Panics
///
/// Panics if help or version flags were disabled.
pub fn verify<C: clap::CommandFactory>() {
    C::command().debug_assert();
    assert_help_enabled(&C::command());
}

/// # Panics
///
/// Panics if `disable_help_flag` or `disable_version_flag` is set.
pub fn assert_help_enabled(command: &Command) {
    assert!(
        !command.is_disable_help_flag_set(),
        "disable_help_flag is forbidden; ctl-core owns -h/--help"
    );
    assert!(
        !command.is_disable_version_flag_set(),
        "disable_version_flag is forbidden; ctl-core owns -V/--version"
    );
}

#[cfg(test)]
mod tests {
    use clap::{Command, CommandFactory, Parser};

    use super::{apply_defaults, assert_help_enabled};

    #[derive(Parser)]
    #[command(version, about = "toy", arg_required_else_help = true)]
    struct Toy {
        #[command(subcommand)]
        command: ToyCmd,
    }

    #[derive(clap::Subcommand)]
    enum ToyCmd {
        Status,
    }

    #[test]
    fn verify_toy() {
        super::verify::<Toy>();
    }

    #[test]
    fn derive_keeps_help_and_version() {
        let mut command = apply_defaults(Toy::command());
        assert_help_enabled(&command);
        assert!(command.is_args_override_self());
        let help = command.render_long_help().to_string();
        assert!(help.contains("-h, --help"));
        assert!(help.contains("-V, --version"));
    }

    #[test]
    #[should_panic(expected = "disable_help_flag is forbidden")]
    fn rejects_disabled_help() {
        let command = Command::new("x").disable_help_flag(true);
        assert_help_enabled(&command);
    }
}

//! Parser defaults. `-h` / `--help` and `-V` / `--version` stay on.

use clap::{ColorChoice, Command};

/// Apply the *ctl parser contract to a clap command.
///
/// `-h/--help` and `-V/--version` stay on. `disable_help_flag` is forbidden.
#[must_use]
pub fn apply_defaults(command: Command) -> Command {
    assert_help_enabled(&command);
    command
        .arg_required_else_help(true)
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

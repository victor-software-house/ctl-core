//! The parser rules every CLI built on ctl-core keeps: `-h` / `--help`, the
//! `help` subcommand, and `-V` / `--version` stay on, and a repeated flag
//! takes its last value.
//!
//! [`apply_defaults`] sets those rules on a Clap command, and [`verify`] is
//! the test a CLI runs over its own parser.
//!
//! ctl-core declares `-h` / `--help` and `help` itself, so they work without
//! Clap's `help` feature, which ctl-core leaves off. Clap turns both off
//! without that feature ([clap_builder 4.6.6, `Command::_build_self`][gate]).
//! ctl-core renders the help, so Clap's help writer is never needed. A plain
//! `Cli::parse()` that skips ctl-core's entry point gets Clap's minimal
//! output for `--help` and no `help` subcommand.
//!
//! [gate]: https://docs.rs/crate/clap_builder/4.6.6/source/src/builder/command.rs#4415
//!
//! ```
//! use clap::Parser;
//!
//! #[derive(Parser)]
//! #[command(version, about = "toy")]
//! struct Cli {}
//!
//! ctl_core::parser::verify::<Cli>();
//! ```

#[cfg(feature = "help")]
use std::ffi::OsString;

#[cfg(feature = "help")]
use clap::ArgMatches;
use clap::{Arg, ArgAction, Command};

#[cfg(feature = "help")]
use crate::{ColorMode, OutputFormat};

/// Output policy recovered with the authoritative Clap graph before a full
/// parse succeeds.
#[cfg(feature = "help")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ParsedOutput {
    pub(crate) format: OutputFormat,
    pub(crate) color: ColorMode,
}

/// Best-effort output policy for help and parse errors.
///
/// Clap still owns option placement, attached short values, domain option
/// values, global propagation, and `--` semantics.
#[cfg(feature = "help")]
pub(crate) fn parsed_output<C: clap::CommandFactory>(raw: &[OsString]) -> ParsedOutput {
    let command = C::command()
        .arg_required_else_help(false)
        .args_override_self(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .ignore_errors(true);
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

/// Whether this invocation asks for help: `-h` or `--help` where Clap reads
/// a flag, or the `help` subcommand ctl-core declares.
///
/// Clap's own `help` subcommand stopped the parse before the parent's
/// required arguments were checked. The probe keeps that: a subcommand
/// lifts its parents' requirements, so `help` still answers when the parent
/// needs an argument.
#[cfg(feature = "help")]
pub(crate) fn wants_help<C: clap::CommandFactory>(raw: &[OsString]) -> bool {
    let declared = C::command();
    let probe = lift_requirements(apply_defaults(declared.clone()));
    match probe.try_get_matches_from(raw) {
        Ok(matches) => names_help(&declared, &matches),
        Err(error) => error.kind() == clap::error::ErrorKind::DisplayHelp,
    }
}

/// `command` with every parent's requirements lifted once a subcommand is
/// present.
#[cfg(feature = "help")]
fn lift_requirements(command: Command) -> Command {
    command
        .subcommand_negates_reqs(true)
        .mut_subcommands(lift_requirements)
}

/// Whether `matches` select the `help` subcommand ctl-core declares, at any
/// depth. A `help` subcommand the CLI declares itself is its own.
#[cfg(feature = "help")]
fn names_help(declared: &Command, matches: &ArgMatches) -> bool {
    let Some((name, selected)) = matches.subcommand() else {
        return false;
    };
    match declared.find_subcommand(name) {
        Some(next) => names_help(next, selected),
        None => name == "help",
    }
}

/// Whether the command's Clap grammar rejects a bare invocation with help.
#[cfg(feature = "app")]
pub(crate) fn requires_input<C: clap::CommandFactory>() -> bool {
    let command = C::command();
    command.is_arg_required_else_help_set() || command.is_subcommand_required_set()
}

/// Apply ctl-core's parser rules to a Clap command and return it.
///
/// `-h/--help`, the `help` subcommand, and `-V/--version` stay on. Repeated
/// flags take the last value (`args_override_self`), and
/// [`chassis_warnings`](crate::flags::chassis_warnings) still
/// reports the clash. [`App`] applies it to every run, so a CLI calls it only
/// to parse a command itself.
///
/// ctl-core declares `-h/--help` and `help` on every command, as Clap does
/// with its `help` feature. A CLI that parses the result itself calls
/// [`help::try_emit`] first: the derive's `from_arg_matches` does not know
/// the `help` subcommand.
///
/// # Panics
///
/// Panics if `command` disabled the help or version flag. ctl-core owns
/// `-h`/`--help` and `-V`/`--version`, so the CLI never turns them off.
#[cfg_attr(feature = "app", doc = "", doc = "[`App`]: crate::App")]
#[cfg_attr(not(feature = "app"), doc = "", doc = "[`App`]: crate#cargo-features")]
#[cfg_attr(
    feature = "help",
    doc = "",
    doc = "[`help::try_emit`]: crate::help::try_emit"
)]
#[cfg_attr(
    not(feature = "help"),
    doc = "",
    doc = "[`help::try_emit`]: crate#cargo-features"
)]
#[must_use]
pub fn apply_defaults(command: Command) -> Command {
    assert_help_enabled(&command);
    with_help(command.args_override_self(true).disable_version_flag(false))
}

/// What Clap's own `help` subcommand says it does.
const HELP_ABOUT: &str = "Print this message or the help of the given subcommand(s)";

/// Whether `command` is a `help` subcommand ctl-core generated, by its name
/// and the text Clap gives its own.
#[cfg(feature = "surface-model")]
pub(crate) fn is_generated_help(command: &Command) -> bool {
    command.get_name() == "help"
        && command
            .get_about()
            .is_some_and(|about| about.to_string() == HELP_ABOUT)
}

/// `command` with ctl-core's `-h/--help` flag and `help` subcommand, at every
/// level Clap would add them, worded as Clap words them.
///
/// A command that already has a `help` argument or subcommand, or turned
/// either off, keeps what it has, so a second call changes nothing.
pub(crate) fn with_help(command: Command) -> Command {
    with_help_below(command, false)
}

/// [`with_help`] for a command whose ancestors pass down a global argument
/// with long help when `inherited_long` is true. Clap copies such an
/// argument into each descendant, where it counts toward long help.
fn with_help_below(mut command: Command, inherited_long: bool) -> Command {
    let long = inherited_long || has_long_help(&command);
    let passed_down = inherited_long
        || command
            .get_arguments()
            .any(|arg| arg.is_global_set() && shows_long_help(arg));
    let help_subcommand = (command.has_subcommands()
        && !command.is_disable_help_subcommand_set()
        && command.find_subcommand("help").is_none())
    .then(|| help_subcommand(&command));
    if !command.is_disable_help_flag_set()
        && command.get_arguments().all(|arg| arg.get_id() != "help")
    {
        command = command.disable_help_flag(true).arg(help_flag(long));
    }
    command = command.mut_subcommands(|subcommand| with_help_below(subcommand, passed_down));
    match help_subcommand {
        Some(help) => command.disable_help_subcommand(true).subcommand(help),
        None => command,
    }
}

/// The `-h/--help` flag, with the text Clap gives its own.
fn help_flag(long: bool) -> Arg {
    let flag = Arg::new("help")
        .short('h')
        .long("help")
        .action(ArgAction::Help);
    if long {
        flag.help("Print help (see more with '--help')")
            .long_help("Print help (see a summary with '-h')")
    } else {
        flag.help("Print help")
    }
}

/// The `help` subcommand, holding a copy of every subcommand name below
/// `parent`, as Clap's own does when a command is built. `help status`
/// parses, `help unknown` fails as an unknown subcommand, and completion
/// offers the names.
fn help_subcommand(parent: &Command) -> Command {
    let help_help = Command::new("help")
        .about(HELP_ABOUT)
        .disable_help_flag(true)
        .disable_version_flag(true);
    Command::new("help")
        .about(HELP_ABOUT)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .subcommands(parent.get_subcommands().map(copy_for_help))
        .subcommand(help_help)
}

/// A subcommand's name, about, visibility, and children, without arguments.
fn copy_for_help(command: &Command) -> Command {
    let copy = Command::new(command.get_name().to_owned())
        .hide(command.is_hide_set())
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .subcommands(command.get_subcommands().map(copy_for_help));
    match command.get_about() {
        Some(about) => copy.about(about.clone()),
        None => copy,
    }
}

/// Whether `command` has long help of its own, by Clap 4.6.6's rule
/// ([`Command::long_help_exists`][rule]). Subcommands do not count.
///
/// [rule]: https://docs.rs/crate/clap_builder/4.6.6/source/src/builder/command.rs#5160
fn has_long_help(command: &Command) -> bool {
    command.get_long_about().is_some()
        || command.get_before_long_help().is_some()
        || command.get_after_long_help().is_some()
        || command.get_arguments().any(shows_long_help)
}

/// Whether `arg` makes its command's long help differ from the short one.
fn shows_long_help(arg: &Arg) -> bool {
    !arg.is_hide_set()
        && (arg.get_long_help().is_some()
            || arg.is_hide_long_help_set()
            || arg.is_hide_short_help_set()
            || (!arg.is_hide_possible_values_set()
                && arg
                    .get_possible_values()
                    .iter()
                    .any(|value| !value.is_hide_set() && value.get_help().is_some())))
}

/// Check a CLI's parser: Clap's recommended `Command::debug_assert`, and that
/// help and version stay on.
///
/// Call it from a unit test, so a mistake in the Clap types fails the test
/// suite and not a user's first run.
///
/// # Panics
///
/// Panics when Clap's `debug_assert` finds an invalid command definition, or
/// when the command disables `-h/--help` or `-V/--version`.
pub fn verify<C: clap::CommandFactory>() {
    C::command().debug_assert();
    assert_help_enabled(&C::command());
}

/// The help-flag contract for one command.
///
/// # Panics
///
/// Panics if `disable_help_flag` or `disable_version_flag` is set.
pub(crate) fn assert_help_enabled(command: &Command) {
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
        assert!(command.is_args_override_self());
        command.build();
        let flags = command
            .get_arguments()
            .filter_map(|arg| arg.get_short().zip(arg.get_long()))
            .collect::<Vec<_>>();
        assert!(flags.contains(&('h', "help")), "{flags:?}");
        assert!(flags.contains(&('V', "version")), "{flags:?}");
        let status = command.find_subcommand("status").map(|status| {
            status
                .get_arguments()
                .any(|arg| arg.get_short() == Some('h'))
        });
        assert_eq!(status, Some(true));
        assert!(command.find_subcommand("help").is_some());
    }

    #[test]
    fn defaults_apply_once() {
        let once = apply_defaults(Toy::command());
        let twice = super::with_help(once.clone());
        assert_eq!(once.get_arguments().count(), twice.get_arguments().count());
        assert_eq!(
            once.get_subcommands().count(),
            twice.get_subcommands().count()
        );
    }

    #[test]
    fn preserves_bare_invocation_policy() {
        let command = apply_defaults(Toy::command().arg_required_else_help(false));
        assert!(!command.is_arg_required_else_help_set());
    }

    #[test]
    #[should_panic(expected = "disable_help_flag is forbidden")]
    fn rejects_disabled_help() {
        let command = Command::new("x").disable_help_flag(true);
        assert_help_enabled(&command);
    }
}

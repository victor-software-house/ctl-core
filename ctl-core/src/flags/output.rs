use std::mem;

use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches, Id};

use crate::color::ColorMode;
use crate::flags::resolve_color;
use crate::format::{FormatName, FormatParser};
#[cfg(feature = "view")]
use crate::view;

/// Output flags for the root parser: format, colour, and quiet. Flatten it with
/// `#[command(flatten)]`.
///
/// `--json` is `--format json`: it sets [`Self::format`]. Naming it beside a
/// `--format` other than `json` is a usage error. Either one on the command
/// line wins over the other's variable, and two variables that name different
/// formats are a usage error.
///
/// [`Self::format`] holds what `--format` and `--json` name, or `None` when
/// neither did, so a default stays open. With `App`, the format in force is
/// `View::format`: it also reads the shorthands of every registered format, and
/// the registry's defaults.
///
/// The flags are global, so a subcommand accepts them too. [`Self::view`]
/// builds the [`View`] for the parsed flags. A CLI that already uses `-c`
/// flattens [`ColorLong`](super::ColorLong) plus [`FormatArgs`] instead.
#[cfg_attr(
    feature = "view",
    doc = "",
    doc = "[`Self::view`]: Self::view",
    doc = "[`View`]: crate::view::View"
)]
#[cfg_attr(
    not(feature = "view"),
    doc = "",
    doc = "[`Self::view`]: crate#cargo-features",
    doc = "[`View`]: crate#cargo-features"
)]
///
/// The colour flags are private, so [`Self::color`] is their only reader and
/// `--no-color` is never skipped. A flag can join the struct in a minor
/// release.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct OutputArgs {
    /// The format `-f/--format` names, or `json` under `--json`.
    pub format: Option<FormatName>,
    color: ColorMode,
    no_color: bool,
    /// Suppress successful pretty output: `-q/--quiet`.
    pub quiet: bool,
}

impl OutputArgs {
    /// The colour mode in force: [`ColorMode::Never`] when `--no-color` is
    /// set, otherwise the `--color` value.
    #[must_use]
    pub fn color(&self) -> ColorMode {
        resolve_color(self.color, self.no_color)
    }

    /// Build the [`View`](crate::view::View) for these flags: the format they
    /// request, the quiet setting, and the colour mode in force. Without a
    /// requested format the view takes the default when it resolves.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self) -> view::View {
        view::View::requested(self.format, self.color()).quiet(self.quiet)
    }
}

/// `-f/--format`, `--json`, and `-q/--quiet`, with no colour flags.
/// `--json` sets [`Self::format`] as it does on [`OutputArgs`].
///
/// Pair it with [`ColorLong`](super::ColorLong) when the CLI owns `-c`, and
/// pass the colour mode to [`Self::view`].
#[cfg_attr(feature = "view", doc = "", doc = "[`Self::view`]: Self::view")]
#[cfg_attr(
    not(feature = "view"),
    doc = "",
    doc = "[`Self::view`]: crate#cargo-features"
)]
///
/// A flag can join it in a minor release.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FormatArgs {
    /// The format `-f/--format` names, or `json` under `--json`.
    pub format: Option<FormatName>,
    /// Suppress successful pretty output: `-q/--quiet`.
    pub quiet: bool,
}

impl FormatArgs {
    /// Build the [`View`](crate::view::View) for these flags, with `color` as
    /// the colour mode and the parsed format and quiet setting.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self, color: ColorMode) -> view::View {
        view::View::requested(self.format, color).quiet(self.quiet)
    }
}

/// `--format`, `--json`, and `-q`/`--quiet` without the `-f` short flag, for
/// CLIs that already use `-f` for a domain flag such as `--file`. `--json`
/// sets [`Self::format`] as it does on [`OutputArgs`].
///
/// A flag can join it in a minor release.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FormatLong {
    /// The format `--format` names, or `json` under `--json`.
    pub format: Option<FormatName>,
    /// Suppress successful pretty output: `-q/--quiet`.
    pub quiet: bool,
}

impl FormatLong {
    /// Build the [`View`](crate::view::View) for these flags, with `color` as
    /// the colour mode and the parsed format and quiet setting.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self, color: ColorMode) -> view::View {
        view::View::requested(self.format, color).quiet(self.quiet)
    }
}

/// The `--json` flag: a shorthand for `--format json`.
///
/// The flag structs add it in `augment_args` and read it from the matches, so
/// a format that declares a shorthand needs no field of its own.
fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .action(ArgAction::SetTrue)
        .global(true)
        .help("Shorthand for `--format json`")
        .help_heading("Output")
}

/// Whether `--json`, or its variable, is set.
fn json_set(matches: &ArgMatches) -> bool {
    matches
        .try_get_one::<bool>("json")
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false)
}

/// The format `--format` and `--json` name together, or `None` when neither
/// does.
///
/// A command-line value outranks a variable, so `--format` on the command line
/// beats `<PREFIX>_JSON`, and `--json` beats `<PREFIX>_FORMAT`. Both on the
/// command line, or both from variables, must name the same format. Clap
/// applies neither `conflicts_with` nor `default_value_if` to a global flag a
/// subcommand repeats, so the two flags are reconciled here.
///
/// # Errors
///
/// An `ArgumentConflict` error when the command line holds `--json` and a
/// `--format` other than `json`, or when the variables of the two name
/// different formats.
fn resolved_format(
    format: Option<FormatName>,
    json: bool,
    matches: &ArgMatches,
) -> Result<Option<FormatName>, clap::Error> {
    let Some(format) = format else {
        return Ok(json.then_some(FormatName::JSON));
    };
    if !json || format == FormatName::JSON {
        return Ok(Some(format));
    }
    let source = |id: &str| matches.value_source(id);
    match (source("format"), source("json")) {
        (Some(ValueSource::CommandLine), Some(ValueSource::CommandLine)) => Err(clap::Error::raw(
            ErrorKind::ArgumentConflict,
            "the argument '--json' cannot be used with '--format <FORMAT>'\n",
        )),
        (Some(ValueSource::CommandLine), _) => Ok(Some(format)),
        (_, Some(ValueSource::CommandLine)) => Ok(Some(FormatName::JSON)),
        _ => Err(clap::Error::raw(
            ErrorKind::ArgumentConflict,
            "the variables for --format and --json name different formats\n",
        )),
    }
}

/// Clap's traits for a public flag struct through its derived twin `$flags`,
/// which turns into `$public` with `resolve` and back with `From`.
macro_rules! through_flags {
    ($public:ident, $flags:ident) => {
        impl FromArgMatches for $public {
            fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
                $flags::from_arg_matches(matches)?.resolve(matches)
            }

            fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
                let mut flags = $flags::from(mem::take(self));
                flags.update_from_arg_matches(matches)?;
                *self = flags.resolve(matches)?;
                Ok(())
            }
        }

        impl Args for $public {
            fn group_id() -> Option<Id> {
                $flags::group_id()
            }

            fn augment_args(command: Command) -> Command {
                $flags::augment_args(command).arg(json_arg())
            }

            fn augment_args_for_update(command: Command) -> Command {
                $flags::augment_args_for_update(command).arg(json_arg())
            }
        }
    };
}

/// The Clap shape of [`OutputArgs`]. [`OutputArgs`] is built from it after the
/// parse, so its `format` reads `--json`, which `augment_args` adds.
#[derive(Args, Default)]
#[command(about = None, long_about = None)]
#[group(id = "OutputArgs")]
struct OutputFlags {
    /// Output representation.
    #[arg(
        short = 'f',
        long,
        global = true,
        value_parser = FormatParser::builtin(),
        help_heading = "Output"
    )]
    format: Option<FormatName>,
    /// Pretty-output colour policy. JSON never contains ANSI.
    #[arg(
        short = 'c',
        long,
        global = true,
        value_enum,
        default_value = "auto",
        help_heading = "Output"
    )]
    color: ColorMode,
    /// Force colourless pretty output. Wins over `--color`.
    #[arg(long, global = true, help_heading = "Output")]
    no_color: bool,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    quiet: bool,
}

impl OutputFlags {
    fn resolve(self, matches: &ArgMatches) -> Result<OutputArgs, clap::Error> {
        Ok(OutputArgs {
            format: resolved_format(self.format, json_set(matches), matches)?,
            color: self.color,
            no_color: self.no_color,
            quiet: self.quiet,
        })
    }
}

impl From<OutputArgs> for OutputFlags {
    fn from(args: OutputArgs) -> Self {
        Self {
            format: args.format,
            color: args.color,
            no_color: args.no_color,
            quiet: args.quiet,
        }
    }
}

through_flags!(OutputArgs, OutputFlags);

/// The Clap shape of [`FormatArgs`]. [`FormatArgs`] is built from it after the
/// parse, so its `format` reads `--json`, which `augment_args` adds.
#[derive(Args, Default)]
#[command(about = None, long_about = None)]
#[group(id = "FormatArgs")]
struct FormatFlags {
    /// Output representation.
    #[arg(
        short = 'f',
        long,
        global = true,
        value_parser = FormatParser::builtin(),
        help_heading = "Output"
    )]
    format: Option<FormatName>,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    quiet: bool,
}

impl FormatFlags {
    fn resolve(self, matches: &ArgMatches) -> Result<FormatArgs, clap::Error> {
        Ok(FormatArgs {
            format: resolved_format(self.format, json_set(matches), matches)?,
            quiet: self.quiet,
        })
    }
}

impl From<FormatArgs> for FormatFlags {
    fn from(args: FormatArgs) -> Self {
        Self {
            format: args.format,
            quiet: args.quiet,
        }
    }
}

through_flags!(FormatArgs, FormatFlags);

/// The Clap shape of [`FormatLong`]. [`FormatLong`] is built from it after the
/// parse, so its `format` reads `--json`, which `augment_args` adds.
#[derive(Args, Default)]
#[command(about = None, long_about = None)]
#[group(id = "FormatLong")]
struct FormatLongFlags {
    /// Output representation.
    #[arg(
        long,
        global = true,
        value_parser = FormatParser::builtin(),
        help_heading = "Output"
    )]
    format: Option<FormatName>,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    quiet: bool,
}

impl FormatLongFlags {
    fn resolve(self, matches: &ArgMatches) -> Result<FormatLong, clap::Error> {
        Ok(FormatLong {
            format: resolved_format(self.format, json_set(matches), matches)?,
            quiet: self.quiet,
        })
    }
}

impl From<FormatLong> for FormatLongFlags {
    fn from(args: FormatLong) -> Self {
        Self {
            format: args.format,
            quiet: args.quiet,
        }
    }
}

through_flags!(FormatLong, FormatLongFlags);

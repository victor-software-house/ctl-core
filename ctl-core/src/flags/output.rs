use std::mem;

use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{ArgMatches, Args, Command, FromArgMatches, Id};

use crate::color::ColorMode;
use crate::flags::resolve_color;
use crate::format::OutputFormat;
#[cfg(feature = "view")]
use crate::view;

/// Output flags for the root parser: format, colour, and quiet. Flatten it with
/// `#[command(flatten)]`.
///
/// `--json` is `--format json`: it sets [`Self::format`]. Naming it beside a
/// `--format` other than `json` is a usage error. Either one on the command
/// line wins over the other's variable, and `<PREFIX>_JSON` wins over
/// `<PREFIX>_FORMAT`.
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
    /// Output representation: `-f/--format`, or `json` under `--json`.
    pub format: OutputFormat,
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

    /// Build the [`View`](crate::view::View) for these flags: the format, the
    /// quiet setting, and the colour mode in force.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self) -> view::View {
        view::View::new(self.format, self.color()).quiet(self.quiet)
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
    /// Output representation: `-f/--format`, or `json` under `--json`.
    pub format: OutputFormat,
    /// Suppress successful pretty output: `-q/--quiet`.
    pub quiet: bool,
}

impl FormatArgs {
    /// Build the [`View`](crate::view::View) for these flags, with `color` as
    /// the colour mode and the parsed format and quiet setting.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self, color: ColorMode) -> view::View {
        view::View::new(self.format, color).quiet(self.quiet)
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
    /// Output representation: `--format`, or `json` under `--json`.
    pub format: OutputFormat,
    /// Suppress successful pretty output: `-q/--quiet`.
    pub quiet: bool,
}

impl FormatLong {
    /// Build the [`View`](crate::view::View) for these flags, with `color` as
    /// the colour mode and the parsed format and quiet setting.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self, color: ColorMode) -> view::View {
        view::View::new(self.format, color).quiet(self.quiet)
    }
}

/// `format`, or JSON when `--json` is set and the command line does not
/// name another format.
///
/// A command-line value outranks a variable, so `--format` on the command line
/// beats `<PREFIX>_JSON`, and `--json` beats `<PREFIX>_FORMAT`. Clap applies
/// neither `conflicts_with` nor `default_value_if` to a global flag a
/// subcommand repeats, so the two flags are reconciled here.
///
/// # Errors
///
/// An `ArgumentConflict` error when the command line holds `--json` and a
/// `--format` other than `json`.
fn resolved_format(
    format: OutputFormat,
    json: bool,
    matches: &ArgMatches,
) -> Result<OutputFormat, clap::Error> {
    let from_line = |id: &str| matches.value_source(id) == Some(ValueSource::CommandLine);
    if !json || (from_line("format") && !from_line("json")) {
        return Ok(format);
    }
    if from_line("format") && format != OutputFormat::Json {
        return Err(clap::Error::raw(
            ErrorKind::ArgumentConflict,
            "the argument '--json' cannot be used with '--format <FORMAT>'\n",
        ));
    }
    Ok(OutputFormat::Json)
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
                $flags::augment_args(command)
            }

            fn augment_args_for_update(command: Command) -> Command {
                $flags::augment_args_for_update(command)
            }
        }
    };
}

/// The Clap shape of [`OutputArgs`]. [`OutputArgs`] is built from it after the
/// parse, so its `format` reads `--json`.
#[derive(Args, Default)]
#[command(about = None, long_about = None)]
#[group(id = "OutputArgs")]
struct OutputFlags {
    /// Output representation.
    #[arg(
        short = 'f',
        long,
        global = true,
        value_enum,
        default_value = "pretty",
        help_heading = "Output"
    )]
    format: OutputFormat,
    /// Shorthand for `--format json`.
    #[arg(long, global = true, help_heading = "Output")]
    json: bool,
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
            format: resolved_format(self.format, self.json, matches)?,
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
            json: false,
            color: args.color,
            no_color: args.no_color,
            quiet: args.quiet,
        }
    }
}

through_flags!(OutputArgs, OutputFlags);

/// The Clap shape of [`FormatArgs`]. [`FormatArgs`] is built from it after the
/// parse, so its `format` reads `--json`.
#[derive(Args, Default)]
#[command(about = None, long_about = None)]
#[group(id = "FormatArgs")]
struct FormatFlags {
    /// Output representation.
    #[arg(
        short = 'f',
        long,
        global = true,
        value_enum,
        default_value = "pretty",
        help_heading = "Output"
    )]
    format: OutputFormat,
    /// Shorthand for `--format json`.
    #[arg(long, global = true, help_heading = "Output")]
    json: bool,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    quiet: bool,
}

impl FormatFlags {
    fn resolve(self, matches: &ArgMatches) -> Result<FormatArgs, clap::Error> {
        Ok(FormatArgs {
            format: resolved_format(self.format, self.json, matches)?,
            quiet: self.quiet,
        })
    }
}

impl From<FormatArgs> for FormatFlags {
    fn from(args: FormatArgs) -> Self {
        Self {
            format: args.format,
            json: false,
            quiet: args.quiet,
        }
    }
}

through_flags!(FormatArgs, FormatFlags);

/// The Clap shape of [`FormatLong`]. [`FormatLong`] is built from it after the
/// parse, so its `format` reads `--json`.
#[derive(Args, Default)]
#[command(about = None, long_about = None)]
#[group(id = "FormatLong")]
struct FormatLongFlags {
    /// Output representation.
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "pretty",
        help_heading = "Output"
    )]
    format: OutputFormat,
    /// Shorthand for `--format json`.
    #[arg(long, global = true, help_heading = "Output")]
    json: bool,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    quiet: bool,
}

impl FormatLongFlags {
    fn resolve(self, matches: &ArgMatches) -> Result<FormatLong, clap::Error> {
        Ok(FormatLong {
            format: resolved_format(self.format, self.json, matches)?,
            quiet: self.quiet,
        })
    }
}

impl From<FormatLong> for FormatLongFlags {
    fn from(args: FormatLong) -> Self {
        Self {
            format: args.format,
            json: false,
            quiet: args.quiet,
        }
    }
}

through_flags!(FormatLong, FormatLongFlags);

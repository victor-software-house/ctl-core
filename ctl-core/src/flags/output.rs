use clap::Args;

use crate::color::ColorMode;
use crate::flags::resolve_color;
use crate::format::OutputFormat;

/// Output flags for the root parser: format, colour, and quiet. Flatten it with
/// `#[command(flatten)]`.
///
/// The flags are global, so a subcommand accepts them too. [`Self::view`]
/// builds the [`View`](crate::view::View) for the parsed flags. A CLI
/// that already uses `-c` flattens [`ColorLong`](super::ColorLong) plus
/// [`FormatArgs`] instead.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
pub struct OutputArgs {
    /// Output representation.
    #[arg(
        short = 'f',
        long,
        global = true,
        value_enum,
        default_value = "pretty",
        help_heading = "Output"
    )]
    pub format: OutputFormat,
    /// Pretty-output colour policy. JSON never contains ANSI.
    #[arg(
        short = 'c',
        long,
        global = true,
        value_enum,
        default_value = "auto",
        help_heading = "Output"
    )]
    pub color: ColorMode,
    /// Force colourless pretty output. Wins over `--color`.
    #[arg(long, global = true, help_heading = "Output")]
    pub no_color: bool,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
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
    pub fn view(&self) -> crate::view::View {
        crate::view::View::new(self.format, self.color()).quiet(self.quiet)
    }
}

/// `-f/--format` and `-q/--quiet`, with no colour flags.
///
/// Pair it with [`ColorLong`](super::ColorLong) when the CLI owns `-c`, and
/// pass the colour mode to [`Self::view`].
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
pub struct FormatArgs {
    /// Output representation.
    #[arg(
        short = 'f',
        long,
        global = true,
        value_enum,
        default_value = "pretty",
        help_heading = "Output"
    )]
    pub format: OutputFormat,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    pub quiet: bool,
}

impl FormatArgs {
    /// Build the [`View`](crate::view::View) for these flags, with `color` as
    /// the colour mode and the parsed format and quiet setting.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self, color: ColorMode) -> crate::view::View {
        crate::view::View::new(self.format, color).quiet(self.quiet)
    }
}

/// `--format` and `-q`/`--quiet` without the `-f` short flag, for CLIs that
/// already use `-f` for a domain flag such as `--file`.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
pub struct FormatLong {
    /// Output representation.
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "pretty",
        help_heading = "Output"
    )]
    pub format: OutputFormat,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    pub quiet: bool,
}

impl FormatLong {
    /// Build the [`View`](crate::view::View) for these flags, with `color` as
    /// the colour mode and the parsed format and quiet setting.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self, color: ColorMode) -> crate::view::View {
        crate::view::View::new(self.format, color).quiet(self.quiet)
    }
}

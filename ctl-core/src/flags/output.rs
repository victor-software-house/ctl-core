use clap::Args;

use crate::color::ColorMode;
use crate::flags::resolve_color;
use crate::format::OutputFormat;
#[cfg(feature = "view")]
use crate::view;

/// Output flags for the root parser: format, colour, and quiet. Flatten it with
/// `#[command(flatten)]`.
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
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
#[non_exhaustive]
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
    color: ColorMode,
    /// Force colourless pretty output. Wins over `--color`.
    #[arg(long, global = true, help_heading = "Output")]
    no_color: bool,
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
    pub fn view(&self) -> view::View {
        view::View::new(self.format, self.color()).quiet(self.quiet)
    }
}

/// `-f/--format` and `-q/--quiet`, with no colour flags.
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
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
#[non_exhaustive]
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
    pub fn view(&self, color: ColorMode) -> view::View {
        view::View::new(self.format, color).quiet(self.quiet)
    }
}

/// `--format` and `-q`/`--quiet` without the `-f` short flag, for CLIs that
/// already use `-f` for a domain flag such as `--file`.
///
/// A flag can join it in a minor release.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
#[non_exhaustive]
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
    pub fn view(&self, color: ColorMode) -> view::View {
        view::View::new(self.format, color).quiet(self.quiet)
    }
}

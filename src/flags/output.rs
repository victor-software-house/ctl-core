use clap::Args;

use crate::color::ColorMode;
use crate::flags::resolve_color;
use crate::format::OutputFormat;

/// Global pretty/JSON output, color, quiet. Flatten onto the root parser.
///
/// `-c/--color` matches forkctl. A consumer that already owns `-c`
/// should flatten [`super::ColorLong`] plus [`FormatArgs`] instead.
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
    /// Pretty-output color policy. JSON never contains ANSI.
    #[arg(
        short = 'c',
        long,
        global = true,
        value_enum,
        default_value = "auto",
        help_heading = "Output"
    )]
    pub color: ColorMode,
    /// Force colorless pretty output. Wins over `--color`.
    #[arg(long, global = true, help_heading = "Output")]
    pub no_color: bool,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    pub quiet: bool,
}

impl OutputArgs {
    /// Effective color after `--no-color`.
    #[must_use]
    pub fn color(&self) -> ColorMode {
        resolve_color(self.color, self.no_color)
    }

    /// Build a [`View`](crate::view::View) from these flags.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self) -> crate::view::View {
        crate::view::View::new(self.format, self.color()).quiet(self.quiet)
    }
}

/// `-f/--format` and `-q/--quiet` without `-c`.
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
    /// Build a [`View`](crate::view::View) with an explicit color policy.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self, color: ColorMode) -> crate::view::View {
        crate::view::View::new(self.format, color).quiet(self.quiet)
    }
}

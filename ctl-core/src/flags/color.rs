use clap::Args;

use crate::color::ColorMode;
use crate::flags::resolve_color;

/// `--color` and `--no-color` without the `-c` short flag, for CLIs that
/// already use `-c` for config.
///
/// [`Self::color`] gives the colour mode in force.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
pub struct ColorLong {
    /// Pretty-output colour policy. JSON never contains ANSI.
    #[arg(
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
}

impl ColorLong {
    /// The colour mode in force: [`ColorMode::Never`] when `--no-color` is
    /// set, otherwise the `--color` value.
    #[must_use]
    pub fn color(&self) -> ColorMode {
        resolve_color(self.color, self.no_color)
    }
}

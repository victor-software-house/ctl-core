use clap::Args;

use crate::color::ColorMode;
use crate::flags::resolve_color;

/// `--color` without `-c`, for CLIs that already use `-c` for config.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
pub struct ColorLong {
    /// Pretty-output color policy. JSON never contains ANSI.
    #[arg(
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
}

impl ColorLong {
    /// Effective color after `--no-color`.
    #[must_use]
    pub fn color(&self) -> ColorMode {
        resolve_color(self.color, self.no_color)
    }
}

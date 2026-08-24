//! Shared clap flatten structs.

mod color;
mod dry_run;
mod output;
mod warn;

pub use color::ColorLong;
pub use dry_run::DryRunArgs;
pub use output::{FormatArgs, FormatLong, OutputArgs};
pub use warn::{FlagWarning, WarningKind, chassis_warnings, emit_warnings, warn_opposites};

use crate::color::ColorMode;

/// `--no-color` wins over a `--color` value.
#[must_use]
pub fn resolve_color(color: ColorMode, no_color: bool) -> ColorMode {
    if no_color { ColorMode::Never } else { color }
}

/// Last-wins boolean after clap `overrides_with`. `on` is the yes flag.
#[must_use]
pub fn switch(on: bool, _off: bool) -> bool {
    on
}

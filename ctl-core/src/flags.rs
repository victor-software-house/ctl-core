//! The flags every ctl CLI shares, as Clap structs to flatten into a parser.
//!
//! - [`OutputArgs`] adds `-f/--format`, `-c/--color`, `--no-color`, and
//!   `-q/--quiet`. [`FormatArgs`], [`FormatLong`], and [`ColorLong`] are the
//!   variants for a CLI that already owns `-c` or `-f`.
//! - [`DryRunArgs`] adds `-n/--dry-run`, with `--preview` as an alias.
//! - [`EnvPrefix`] binds every long option to an environment variable.
//! - [`chassis_warnings`] and [`warn_opposites`] find repeated or contradictory
//!   flags, which [`emit_warnings`] prints.
//!
//! ```
//! use clap::Parser;
//! use ctl_core::flags::OutputArgs;
//! use ctl_core::{ColorMode, OutputFormat};
//!
//! #[derive(Parser)]
//! struct Cli {
//!     #[command(flatten)]
//!     output: OutputArgs,
//! }
//!
//! let cli = Cli::parse_from(["toy", "--format", "json", "--no-color"]);
//! assert_eq!(cli.output.format, OutputFormat::Json);
//! assert_eq!(cli.output.color(), ColorMode::Never);
//! ```

mod color;
mod dry_run;
mod env;
mod output;
mod warn;

pub use color::ColorLong;
pub use dry_run::DryRunArgs;
pub use env::{EnvPrefix, env_under_flags};
pub use output::{FormatArgs, FormatLong, OutputArgs};
pub use warn::{FlagWarning, WarningKind, chassis_warnings, emit_warnings, warn_opposites};

use crate::color::ColorMode;

/// The colour mode in force: [`ColorMode::Never`] when `no_color` is set,
/// otherwise `color`.
#[must_use]
pub fn resolve_color(color: ColorMode, no_color: bool) -> ColorMode {
    if no_color { ColorMode::Never } else { color }
}

/// Last-wins boolean after clap `overrides_with`. `on` is the yes flag.
///
/// Clap clears the flag that lost, so `on` already holds the answer.
#[must_use]
pub fn switch(on: bool) -> bool {
    on
}

//! The flags every CLI built on ctl-core shares, as Clap structs to flatten
//! into a parser.
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
//! use ctl_core::color::ColorMode;
//! use ctl_core::flags::OutputArgs;
//! use ctl_core::format::OutputFormat;
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
pub use warn::{FlagClash, FlagWarning, chassis_warnings, emit_warnings, warn_opposites};

use crate::color::ColorMode;

/// The colour mode in force: [`ColorMode::Never`] when `no_color` is set,
/// otherwise `color`.
#[must_use]
pub fn resolve_color(color: ColorMode, no_color: bool) -> ColorMode {
    if no_color { ColorMode::Never } else { color }
}

/// The value of a `--x`/`--no-x` pair declared with Clap's `overrides_with`
/// both ways: `on`, the `--x` flag, returned unchanged.
///
/// Clap clears the flag that came first, so `on` already holds the answer.
/// The call names that rule at the call site, so a reader does not look for
/// a missing check of `--no-x`.
#[must_use]
pub fn switch(on: bool) -> bool {
    on
}

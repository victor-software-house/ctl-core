//! Import once in a *ctl `main.rs`.
//!
//! ```ignore
//! use ctl_core::prelude::*;
//!
//! fn main() -> ExitCode {
//!     go::<Cli, _>("toy", |cli| {
//!         let view = cli.format.view(cli.color.color());
//!         view.show(&report)
//!     })
//! }
//! ```

pub use std::process::ExitCode;

pub use clap::{Args, Parser, Subcommand};
pub use indoc::{concatdoc, eprintdoc, formatdoc, indoc, printdoc, writedoc};

pub use crate::color::ColorMode;
#[cfg(feature = "cli")]
pub use crate::flags::{
    ColorLong, DryRunArgs, FormatArgs, OutputArgs, chassis_warnings, emit_warnings, warn_opposites,
};
pub use crate::format::OutputFormat;
pub use crate::model::{Envelope, ErrorBody, SCHEMA_VERSION};
#[cfg(feature = "cli")]
pub use crate::parser::verify;
#[cfg(feature = "cli")]
pub use crate::run::{go, main_with_help};
#[cfg(feature = "view")]
pub use crate::table::{grid, kv};
#[cfg(feature = "usage")]
pub use crate::usage::{mount_line, spec, spec_bin, take};
#[cfg(feature = "view")]
pub use crate::view::{Pretty, Render, View, render_template};

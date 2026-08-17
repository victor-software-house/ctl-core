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
pub use indoc::{formatdoc, indoc, writedoc};

pub use crate::color::ColorMode;
#[cfg(feature = "cli")]
pub use crate::flags::{ColorLong, DryRunArgs, FormatArgs, OutputArgs};
pub use crate::format::OutputFormat;
pub use crate::model::{Envelope, ErrorBody, SCHEMA_VERSION};
#[cfg(feature = "cli")]
pub use crate::parser::verify;
#[cfg(feature = "cli")]
pub use crate::run::{go, main_with_help};
#[cfg(feature = "view")]
pub use crate::view::{Render, View};

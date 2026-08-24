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

#[cfg(feature = "cli")]
pub use clap::{Args, Parser, Subcommand};
pub use indoc::{concatdoc, eprintdoc, formatdoc, indoc, printdoc, writedoc};

#[cfg(feature = "app")]
pub use crate::app::App;
pub use crate::color::ColorMode;
#[cfg(feature = "document")]
pub use crate::document::{
    Document, Fields, Notice, NoticeLevel, Role, Rule, Section, Table, Text,
};
#[cfg(feature = "cli")]
pub use crate::flags::{
    ColorLong, DryRunArgs, FormatArgs, FormatLong, OutputArgs, chassis_warnings, emit_warnings,
    warn_opposites,
};
pub use crate::format::OutputFormat;
pub use crate::model::{Envelope, ErrorBody, SCHEMA_VERSION};
#[cfg(feature = "cli")]
pub use crate::parser::verify;
#[cfg(feature = "render")]
pub use crate::render::{RenderOptions, Renderer};
#[cfg(feature = "help")]
pub use crate::run::main_with_help;
#[cfg(feature = "surface")]
pub use crate::surface::Surface;
#[cfg(feature = "render")]
pub use crate::table::{grid, kv};
#[cfg(feature = "usage")]
pub use crate::usage::{mount_line, spec, spec_bin, take};
#[cfg(feature = "view")]
pub use crate::view::{Captured, MessageKind, Present, Stream, View};

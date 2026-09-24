//! Import once in a *ctl `main.rs`.
//!
//! ```ignore
//! use ctl_core::prelude::*;
//!
//! fn main() -> ExitCode {
//!     App::<Cli>::new("toy").run(execute)
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
pub use crate::render::{
    DEFAULT_COLUMN_BUFFER, DEFAULT_COLUMN_BUFFER_ENV, DEFAULT_COLUMN_BUFFER_ENVS,
    DEFAULT_FALLBACK_WIDTH, DEFAULT_LIST_STYLE_ENV, DEFAULT_LIST_STYLE_ENVS,
    DEFAULT_MINIMUM_AUTOMATIC_WIDTH, DEFAULT_RECORD_STYLE_ENV, DEFAULT_RECORD_STYLE_ENVS,
    DEFAULT_ROW_SEPARATION_ENV, DEFAULT_ROW_SEPARATION_ENVS, ListStyle, RecordStyle, RenderOptions,
    Renderer, RowSeparation,
};
#[cfg(feature = "surface-model")]
pub use crate::surface::Surface;
#[cfg(feature = "render")]
pub use crate::table::{grid, kv};
#[cfg(feature = "usage")]
pub use crate::usage::{mount_line, spec, spec_bin, take};
#[cfg(feature = "view")]
pub use crate::view::{
    Captured, DEFAULT_JSON_LAYOUT_ENV, DEFAULT_JSON_LAYOUT_ENVS, JsonLayout, MessageKind, Present,
    Stream, View,
};

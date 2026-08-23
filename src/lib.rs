//! Shared clap chassis and presentation kernel for the `*ctl` CLIs.
//!
//! Not a command. Import as [`ctl_core`](crate). Domain handlers return typed
//! data. [`Document`] describes the human view, [`View`] chooses pretty,
//! colorless, or JSON, and the terminal engine remains private to this crate.
//!
//! ## Cargo features
#![doc = document_features::document_features!()]
//!
//! Unused features do not compile `clap`, `comfy-table`, or `schemars`.
//!
//! Crate docs live here. The GitHub README is not rustdoc.

#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![allow(clippy::missing_errors_doc)]

/// Fluent typed CLI lifecycle.
#[cfg(feature = "app")]
pub mod app;
/// Color policy (`auto` / `always` / `never`).
pub mod color;
/// Semantic human presentation independent of a terminal engine.
#[cfg(feature = "document")]
pub mod document;
/// Output representation (`pretty` / `json`).
pub mod format;
/// Schema-first wire types.
pub mod model;

/// Clap flatten structs for shared flags.
#[cfg(feature = "cli")]
pub mod flags;
/// Styled `-h` / `--help` renderer.
#[cfg(feature = "help")]
pub mod help;
#[cfg(feature = "render")]
mod layout;
/// Parser defaults (`-h` / `-V` stay on).
#[cfg(feature = "cli")]
pub mod parser;
/// One-import surface for a *ctl binary (`use ctl_core::prelude::*`).
pub mod prelude;
/// Semantic document renderer.
#[cfg(feature = "render")]
pub mod render;
/// Process `ExitCode` wrapper.
#[cfg(feature = "cli")]
pub mod run;
#[cfg(feature = "render")]
mod style;
/// Compact pretty tables for command output.
#[cfg(feature = "render")]
pub mod table;
/// Mise Usage spec from a clap command.
#[cfg(feature = "usage")]
pub mod usage;
/// Pretty / JSON / colorless emitters.
#[cfg(feature = "view")]
pub mod view;

#[cfg(feature = "app")]
pub use app::App;
pub use color::ColorMode;
#[cfg(feature = "document")]
pub use document::{
    Block, Document, Fields, Notice, NoticeLevel, Role, Rule, Section, Span, Table, Text,
};
#[cfg(feature = "cli")]
pub use flags::{
    ColorLong, DryRunArgs, FlagWarning, FormatArgs, OutputArgs, WarningKind, chassis_warnings,
    emit_warnings, resolve_color, switch, warn_opposites,
};
pub use format::OutputFormat;
pub use indoc::{concatdoc, eprintdoc, formatdoc, indoc, printdoc, writedoc};
pub use model::{Envelope, ErrorBody, SCHEMA_VERSION};
#[cfg(feature = "render")]
pub use render::{RenderOptions, Renderer};
#[cfg(feature = "cli")]
pub use run::main as run;
#[cfg(feature = "render")]
pub use table::{grid, kv};
#[cfg(feature = "usage")]
pub use usage::{mount_line, spec, spec_bin, take};
#[cfg(feature = "view")]
pub use view::{Captured, MessageKind, Present, Stream, View};

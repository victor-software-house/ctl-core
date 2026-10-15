//! Shared clap chassis and presentation kernel for the `*ctl` CLIs. A
//! library, not a command.
//!
//! A consumer owns typed domain requests and results. ctl-core owns parsing,
//! help, Usage mounts, color policy, terminal layout, streams, errors, and
//! JSON emission. One serializable result feeds pretty, colorless, and JSON
//! output.
//!
//! ## Quick start
//!
//! ```no_run
//! use ctl_core::prelude::*;
//! use serde::Serialize;
//!
//! #[derive(Parser)]
//! #[command(version, about = "example")]
//! struct Cli {
//!     #[command(flatten)]
//!     output: OutputArgs,
//!     #[command(subcommand)]
//!     command: Command,
//! }
//!
//! #[derive(Subcommand)]
//! enum Command {
//!     /// Show status.
//!     Status,
//! }
//!
//! #[derive(Serialize)]
//! struct Status {
//!     pending: usize,
//! }
//!
//! impl Present for Status {
//!     fn present(&self) -> Document {
//!         Document::new().fields(Fields::new().row("pending", self.pending.to_string()))
//!     }
//! }
//!
//! fn main() -> ExitCode {
//!     App::<Cli>::new("example")
//!         .view(|cli| cli.output.view())
//!         .run(|cli| match cli.command {
//!             Command::Status => Ok(Status { pending: 0 }),
//!         })
//! }
//! ```
//!
//! The default `app` feature covers this example.
//!
//! ## How output flows
//!
//! ```text
//! Clap types ──→ App ──→ typed domain result ──┬──→ JSON
//!                                              └──→ Document ──→ pretty/colorless
//! ```
//!
//! - Domain handlers return data. They do not print, read terminal state, or
//!   choose the output format.
//! - [`Present`] maps a serializable result to a semantic [`Document`].
//! - [`View`] serializes the result for JSON and renders its document for
//!   pretty and colorless output.
//! - JSON always goes to stdout and never contains ANSI. Human failures go to
//!   stderr.
//! - The terminal engines, Comfy Table, Anstyle, and Anstream, stay private.
//!
//! [`prelude`] is the one import a `*ctl` binary needs.
//!
//! ## Cargo features
//!
//! Features are additive. Unused ones do not compile `clap`, `comfy-table`,
//! or `schemars`.
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

#[cfg(feature = "app")]
pub mod app;
/// Color policy (`auto` / `always` / `never`).
pub mod color;
#[cfg(feature = "document")]
pub mod document;
/// Output representation (`pretty` / `json`).
pub mod format;
pub mod model;

#[cfg(feature = "fixture")]
pub mod fixture;
#[cfg(feature = "cli")]
pub mod flags;
#[cfg(feature = "help")]
pub mod help;
#[cfg(feature = "input")]
pub mod input;
#[cfg(feature = "render")]
mod layout;
#[cfg(feature = "cli")]
pub mod parser;
pub mod prelude;
#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "served")]
pub mod served;
#[cfg(feature = "render")]
mod style;
#[cfg(feature = "surface-model")]
pub mod surface;
#[cfg(feature = "render")]
pub mod table;
#[cfg(feature = "usage")]
pub mod usage;
#[cfg(feature = "validate")]
pub mod validate;
#[cfg(feature = "view")]
pub mod view;

#[cfg(feature = "app")]
#[doc(hidden)]
pub use app::App;
#[doc(hidden)]
pub use color::ColorMode;
#[cfg(feature = "document")]
#[doc(hidden)]
pub use document::{
    Block, Document, Fields, Notice, NoticeLevel, Role, Rule, Section, Span, Table, Text,
};
#[cfg(feature = "cli")]
#[doc(hidden)]
pub use flags::{
    ColorLong, DryRunArgs, EnvPrefix, FlagWarning, FormatArgs, FormatLong, OutputArgs, WarningKind,
    chassis_warnings, emit_warnings, resolve_color, switch, warn_opposites,
};
#[doc(hidden)]
pub use format::OutputFormat;
#[doc(hidden)]
pub use indoc::{concatdoc, eprintdoc, formatdoc, indoc, printdoc, writedoc};
#[doc(hidden)]
pub use model::{Envelope, ErrorBody, SCHEMA_VERSION};
#[cfg(feature = "render")]
#[doc(hidden)]
pub use render::{
    DEFAULT_COLUMN_BUFFER, DEFAULT_COLUMN_BUFFER_ENV, DEFAULT_COLUMN_BUFFER_ENVS,
    DEFAULT_FALLBACK_WIDTH, DEFAULT_LIST_STYLE_ENV, DEFAULT_LIST_STYLE_ENVS,
    DEFAULT_MINIMUM_AUTOMATIC_WIDTH, DEFAULT_RECORD_STYLE_ENV, DEFAULT_RECORD_STYLE_ENVS,
    DEFAULT_ROW_SEPARATION_ENV, DEFAULT_ROW_SEPARATION_ENVS, ListStyle, RecordStyle, RenderOptions,
    Renderer, RowSeparation,
};
#[cfg(feature = "surface-model")]
#[doc(hidden)]
pub use surface::{Surface, SurfaceArgument, SurfaceCommand, SurfaceRequirement, SurfaceScope};
#[cfg(feature = "render")]
#[doc(hidden)]
pub use table::{grid, kv};
#[cfg(feature = "usage")]
#[doc(hidden)]
pub use usage::{mount_directives, mount_line, spec, spec_bin, take};
#[cfg(feature = "view")]
#[doc(hidden)]
pub use view::{
    Captured, DEFAULT_JSON_LAYOUT_ENV, DEFAULT_JSON_LAYOUT_ENVS, JsonLayout, MessageKind, Present,
    Stream, View,
};

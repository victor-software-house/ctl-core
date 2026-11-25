//! A framework for command-line programs built on clap. A library, not a
//! command.
//!
//! A program declares its arguments with clap's derive macros and writes a
//! handler that returns a typed result. ctl-core parses the arguments, prints
//! help, and writes the result as terminal text or as JSON. It also supplies
//! the shared flags, colour policy, error output, and exit codes. One
//! serializable result feeds the pretty, colourless, and JSON output.
//!
//! ## Quick start
//!
//! The derive macros expand to paths in `clap` and `serde`, so a program
//! depends on both itself, each with its `derive` feature:
//!
//! ```text
//! cargo add ctl-core
//! cargo add clap --no-default-features --features std,derive
//! cargo add serde --features derive
//! ```
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
//! ## Where to start
//!
//! - [`App`] runs a CLI: it parses argv, calls your handler, and writes the
//!   result or the error. [`prelude`] is the one ctl-core import a binary
//!   needs.
//! - [`Present`] maps a serializable result to a [`Document`], and its
//!   [`Outcome`] says whether the result succeeded. [`View`] writes it through
//!   a [`Format`]: `pretty` text, colourless text, or `json`, or a format the
//!   CLI registers on [`Formats`].
//! - [`Document`] builds human output from headings, paragraphs, [`Fields`],
//!   [`Table`]s, [`Section`]s, and [`Notice`]s. It has no layout in it.
//! - [`OutputArgs`], [`DryRunArgs`], and [`EnvPrefix`] are the shared flags and
//!   the environment binding. [`ColorMode`] and [`FormatName`] are the values
//!   the flags parse to. A [`FlagWarning`] reports a [`FlagClash`] between two
//!   of them, where the last one given wins.
//! - [`RenderOptions`] and [`Theme`] set how a document renders: the width
//!   policy, the record and table framing, and the style of each role.
//! - [`Envelope`] and [`ErrorBody`] are the JSON error shape. A handler returns
//!   a [`CommandError`] to give a failure its own exit code and a `kind` value
//!   a script can match on.
//! - [`input::Input`] loads a CLI's declared config from YAML or TOML, and
//!   reports every problem on its file and line. [`input::FieldPath`] and
//!   [`input::Input::place`] place a problem of the CLI's own the same way.
//!
//! ## How output flows
//!
//! ```text
//! Clap types ──→ App ──→ typed domain result ──→ Format ──┬──→ data ──→ JSON
//!                                                          └──→ Document ──→ pretty
//! ```
//!
//! - Domain handlers return data. They do not print, read terminal state, or
//!   choose the output format.
//! - [`Present`] maps a serializable result to a semantic [`Document`].
//! - [`View`] hands the result to its [`Format`], which serializes it or
//!   renders its document. `pretty`, `json`, `tsv`, `csv`, and `markdown` are
//!   such formats, and a CLI adds its own on [`Formats`]. With no format named,
//!   a terminal gets `pretty` and a pipe gets `tsv`.
//! - Success goes to stdout and failure to stderr, in JSON as in pretty output.
//!   A machine format, such as JSON, never contains ANSI.
//! - A reader that closes stdout early, such as `head -1`, does not change the
//!   exit code. A command that succeeded exits 0, and a failure keeps its code.
//!   [`App::broken_pipe`] can make every closed pipe a failure.
//!
//! [`prelude`] is the one ctl-core import a program's `main.rs` needs.
//!
//! ## The rendering engine
//!
//! A [`Document`] renders through three companion crates, which a CLI can also
//! use directly through the re-exports [`text`], [`layout`], and
//! [`term`] (`term` with `view` or `help`):
//!
//! - `ctl-text`: styled spans, lines, and text, with widths counted in terminal
//!   cells, wrapping, and truncation. Text from outside the program loses its
//!   control characters when a span is built.
//! - `ctl-layout`: the widgets a document becomes, such as tables, records, and
//!   rules. They lay out plain lines of styled spans and never see an escape
//!   sequence.
//! - `ctl-term`: whether to write colour and how wide to lay out, decided for
//!   the stream being written. Its `Encoder` is the one place an escape
//!   sequence is written.
//!
//! A [`Theme`] maps each [`Role`] to a style on the way from the document to
//! the widgets. [`document::Text`] holds roles and [`text::Text`] holds
//! styles.
//!
//! ## Cargo features
//!
//! Features are additive. Unused ones do not compile `clap`, `schemars`, or
//! the rendering crates. `document` builds documents with no engine.
//! `render` adds `ctl-text`, `ctl-layout`, and `ctl-term`'s encoder, and
//! renders to a `String` without reading any terminal. `help` and `view` turn
//! on `ctl-term`'s `detect` feature, which reads the stream's width and
//! decides colour for it.
//!
//! Declared input takes one feature per format. `input-yaml` reads YAML and
//! `input-toml` reads TOML; neither links the other's parser. `validate`
//! adds garde validation, and `input` is `input-yaml` with `validate`.
#![cfg_attr(
    not(feature = "app"),
    doc = "",
    doc = "[`App`]: crate#cargo-features",
    doc = "[`App::broken_pipe`]: crate#cargo-features",
    doc = "[`CommandError`]: crate#cargo-features"
)]
#![cfg_attr(
    not(feature = "view"),
    doc = "",
    doc = "[`Present`]: crate#cargo-features",
    doc = "[`Outcome`]: crate#cargo-features",
    doc = "[`View`]: crate#cargo-features"
)]
#![cfg_attr(
    not(feature = "document"),
    doc = "",
    doc = "[`Document`]: crate#cargo-features",
    doc = "[`Fields`]: crate#cargo-features",
    doc = "[`Table`]: crate#cargo-features",
    doc = "[`Section`]: crate#cargo-features",
    doc = "[`Notice`]: crate#cargo-features",
    doc = "[`Role`]: crate#cargo-features",
    doc = "[`document::Text`]: crate#cargo-features"
)]
#![cfg_attr(
    not(feature = "view"),
    doc = "",
    doc = "[`Format`]: crate#cargo-features",
    doc = "[`Formats`]: crate#cargo-features"
)]
#![cfg_attr(
    not(feature = "cli"),
    doc = "",
    doc = "[`OutputArgs`]: crate#cargo-features",
    doc = "[`DryRunArgs`]: crate#cargo-features",
    doc = "[`EnvPrefix`]: crate#cargo-features",
    doc = "[`FlagWarning`]: crate#cargo-features",
    doc = "[`FlagClash`]: crate#cargo-features"
)]
#![cfg_attr(
    not(feature = "render"),
    doc = "",
    doc = "[`RenderOptions`]: crate#cargo-features",
    doc = "[`Theme`]: crate#cargo-features",
    doc = "[`text`]: crate#cargo-features",
    doc = "[`layout`]: crate#cargo-features",
    doc = "[`text::Text`]: crate#cargo-features"
)]
#![cfg_attr(
    not(any(feature = "view", feature = "help")),
    doc = "",
    doc = "[`term`]: crate#cargo-features"
)]
#![cfg_attr(
    not(any(feature = "input-yaml", feature = "input-toml")),
    doc = "",
    doc = "[`input::Input`]: crate#cargo-features",
    doc = "[`input::FieldPath`]: crate#cargo-features",
    doc = "[`input::Input::place`]: crate#cargo-features"
)]
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "app")]
pub mod app;
pub mod color;
#[cfg(feature = "completion")]
pub mod completion;
#[cfg(feature = "document")]
pub mod document;
pub mod format;
pub mod model;

#[cfg(feature = "fixture")]
pub mod fixture;
#[cfg(feature = "cli")]
pub mod flags;
#[cfg(feature = "help")]
pub mod help;
#[cfg(any(feature = "input-yaml", feature = "input-toml"))]
pub mod input;
#[cfg(feature = "cli")]
pub mod parser;
pub mod prelude;
#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "served")]
pub mod served;
#[cfg(feature = "shell")]
pub mod shell;
#[cfg(feature = "shell-init")]
pub mod shell_init;
#[cfg(feature = "surface-model")]
pub mod surface;
#[cfg(feature = "render")]
mod theme;
#[cfg(feature = "usage")]
pub mod usage;
#[cfg(feature = "validate")]
pub mod validate;
#[cfg(feature = "render")]
pub use ctl_layout as layout;
#[cfg(any(feature = "view", feature = "help"))]
pub use ctl_term as term;
#[cfg(feature = "render")]
pub use ctl_text as text;
#[cfg(feature = "unsafe-chars")]
pub use ctl_text::unsafe_chars;
#[cfg(feature = "view")]
pub mod view;

#[cfg(feature = "app")]
#[doc(hidden)]
pub use app::{App, BrokenPipe, CommandError};
#[doc(hidden)]
pub use color::ColorMode;
#[cfg(feature = "document")]
#[doc(hidden)]
pub use document::Document;
#[cfg(feature = "document")]
#[doc(hidden)]
pub use document::{Block, Fields, Notice, NoticeLevel, Role, Rule, Section, Span, Table, Text};
#[cfg(feature = "cli")]
#[doc(hidden)]
pub use flags::EnvPrefix;
#[cfg(feature = "cli")]
#[doc(hidden)]
pub use flags::{
    ColorLong, DryRunArgs, FlagClash, FlagWarning, FormatArgs, FormatLong, OutputArgs,
    chassis_warnings, emit_warnings, resolve_color, switch, warn_opposites,
};
#[doc(hidden)]
pub use format::FormatName;
#[cfg(feature = "view")]
#[doc(hidden)]
pub use format::{Format, Formats};
#[doc(hidden)]
pub use indoc::{concatdoc, eprintdoc, formatdoc, indoc, printdoc, writedoc};
#[doc(hidden)]
pub use model::{Envelope, ErrorBody, SCHEMA_VERSION};
#[cfg(any(feature = "view", feature = "help"))]
#[doc(hidden)]
pub use render::{
    DEFAULT_COLUMN_BUFFER, DEFAULT_COLUMN_BUFFER_ENV, DEFAULT_COLUMN_BUFFER_ENVS,
    DEFAULT_MINIMUM_AUTOMATIC_WIDTH,
};
#[cfg(feature = "render")]
#[doc(hidden)]
pub use render::{
    DEFAULT_COLUMN_FLOOR_ENV, DEFAULT_COLUMN_FLOOR_ENVS, DEFAULT_FALLBACK_WIDTH,
    DEFAULT_LIST_STYLE_ENV, DEFAULT_LIST_STYLE_ENVS, DEFAULT_RECORD_STYLE_ENV,
    DEFAULT_RECORD_STYLE_ENVS, DEFAULT_ROW_SEPARATION_ENV, DEFAULT_ROW_SEPARATION_ENVS,
    DEFAULT_TABLE_FIT_ENV, DEFAULT_TABLE_FIT_ENVS, Frame, RecordStyle, RenderOptions,
    RowSeparation, TableFit,
};
#[cfg(feature = "surface-model")]
#[doc(hidden)]
pub use surface::{Surface, SurfaceArgument, SurfaceCommand, SurfaceRequirement, SurfaceScope};
#[cfg(feature = "render")]
pub use theme::Theme;
#[cfg(feature = "usage")]
#[doc(hidden)]
pub use usage::{mount_directives, mount_line, spec, spec_bin, take};
#[cfg(feature = "view")]
#[doc(hidden)]
pub use view::Present;
#[cfg(feature = "view")]
#[doc(hidden)]
pub use view::{
    Captured, DEFAULT_JSON_LAYOUT_ENV, DEFAULT_JSON_LAYOUT_ENVS, Destination, JsonLayout, Outcome,
    View,
};

/// The README's example, compiled as a doctest. It needs the default `app`
/// feature.
#[doc = include_str!("../README.md")]
#[cfg(all(doctest, feature = "app"))]
pub struct ReadmeDoctests;

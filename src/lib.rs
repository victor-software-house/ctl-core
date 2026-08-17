//! Shared clap chassis for the `*ctl` CLIs.
//!
//! Not a command. Import as [`ctl_core`](crate). Models
//! ([`Envelope`], [`ColorMode`], [`OutputFormat`]) come first. [`View`]
//! picks pretty, JSON, or colorless. JSON never contains ANSI.
//!
//! ## Cargo features
#![doc = document_features::document_features!()]
//!
//! Unused features do not compile `clap`, `comfy-table`, or `schemars`.
//!
//! Crate docs live here. The GitHub README is not rustdoc.

#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![allow(clippy::missing_errors_doc)]

/// Color policy (`auto` / `always` / `never`).
pub mod color;
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
/// Terminal width helpers for help tables.
#[cfg(feature = "help")]
pub mod layout;
/// Parser defaults (`-h` / `-V` stay on).
#[cfg(feature = "cli")]
pub mod parser;
/// One-import surface for a *ctl binary (`use ctl_core::prelude::*`).
pub mod prelude;
/// Process `ExitCode` wrapper.
#[cfg(feature = "cli")]
pub mod run;
/// ANSI styles for pretty views.
#[cfg(feature = "color")]
pub mod style;
/// Pretty / JSON / colorless emitters.
#[cfg(feature = "view")]
pub mod view;

pub use color::ColorMode;
#[cfg(feature = "cli")]
pub use flags::{
    ColorLong, DryRunArgs, FlagWarning, FormatArgs, OutputArgs, WarningKind, chassis_warnings,
    emit_warnings, resolve_color, switch, warn_opposites,
};
pub use format::OutputFormat;
pub use indoc::{formatdoc, indoc, writedoc};
pub use model::{Envelope, ErrorBody, SCHEMA_VERSION};
#[cfg(feature = "cli")]
pub use run::main as run;
#[cfg(feature = "view")]
pub use view::{Render, View};

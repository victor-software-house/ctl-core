//! Integration tests that share the `Toy` command fixture, linked as one
//! binary.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]

mod common;
#[cfg(feature = "cli")]
mod flags_cli;
#[cfg(feature = "help")]
mod help_render;

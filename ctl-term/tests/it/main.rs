//! Integration tests for ctl-term: the `terminal-output` spec scenarios that
//! this crate owns, the detection tables, the encoder, the closed pipe, and
//! the escape properties.

mod closed_pipe;
mod common;
mod detection;
mod encoding;
mod palette;
mod properties;
mod scenarios;

//! Integration tests for ctl-term: the `terminal-output` spec scenarios that
//! this crate owns, the detection tables, the encoder, and the escape
//! properties. The closed pipe has its own target, `tests/closed_pipe.rs`.

mod common;
mod detection;
mod encoding;
mod palette;
mod properties;
mod scenarios;

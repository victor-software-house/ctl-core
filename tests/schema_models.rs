//! schemars output for the public enums / envelope.
#![allow(missing_docs)]
#![cfg(feature = "schema")]

use ctl_core::{ColorMode, Envelope, OutputFormat};
use schemars::schema_for;

#[test]
fn color_schema_lists_modes() {
    let text = serde_json::to_string(&schema_for!(ColorMode)).unwrap();
    for token in ["auto", "always", "never"] {
        assert!(text.contains(token), "{text}");
    }
}

#[test]
fn format_schema_lists_views() {
    let text = serde_json::to_string(&schema_for!(OutputFormat)).unwrap();
    assert!(text.contains("pretty") && text.contains("json"), "{text}");
}

#[test]
fn envelope_schema_exists() {
    let text = serde_json::to_string(&schema_for!(Envelope<String>)).unwrap();
    assert!(
        text.contains("status") || text.contains("Envelope"),
        "{text}"
    );
}

//! schemars output for the public enums / envelope.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "schema")]

use ctl_core::{ColorMode, Envelope, ErrorBody, OutputFormat};
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
fn error_body_schema_leaves_the_kind_optional() {
    let schema = serde_json::to_value(schema_for!(ErrorBody)).unwrap();
    assert!(schema["properties"]["kind"].is_object(), "{schema}");
    assert_eq!(schema["required"], serde_json::json!(["bin", "message"]));
}

#[test]
fn envelope_schema_exists() {
    let text = serde_json::to_string(&schema_for!(Envelope<String>)).unwrap();
    assert!(
        text.contains("status") || text.contains("Envelope"),
        "{text}"
    );
}

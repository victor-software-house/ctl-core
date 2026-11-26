//! schemars output for the public enums / envelope.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "schema")]

use ctl_core::{ColorMode, Envelope, ErrorBody};
use schemars::schema_for;

#[test]
fn color_schema_lists_modes() {
    let text = serde_json::to_string(&schema_for!(ColorMode)).unwrap();
    for token in ["auto", "always", "never"] {
        assert!(text.contains(token), "{text}");
    }
}

#[test]
fn error_body_schema_leaves_the_kind_optional() {
    let schema = serde_json::to_value(schema_for!(ErrorBody)).unwrap();
    assert!(schema["properties"]["kind"].is_object(), "{schema}");
    assert_eq!(schema["required"], serde_json::json!(["bin", "message"]));
}

#[test]
fn error_body_schema_lists_each_part_and_requires_none_of_them() {
    let schema = serde_json::to_value(schema_for!(ErrorBody)).unwrap();
    for part in ["causes", "problems", "usage", "parse"] {
        assert!(schema["properties"][part].is_object(), "{part}: {schema}");
    }
    let defs = &schema["$defs"];
    assert_eq!(
        defs["InputProblem"]["required"],
        serde_json::json!(["input", "message"])
    );
    assert_eq!(
        defs["ParseFailure"]["required"],
        serde_json::json!(["reason"])
    );
    for part in [
        "argument",
        "missing",
        "value",
        "valid_values",
        "subcommand",
        "valid_subcommands",
        "suggested_arguments",
        "suggested_values",
        "suggested_subcommands",
        "suggested_commands",
        "conflicts_with",
        "actual_values",
        "expected_values",
        "min_values",
        "trailing_argument",
        "tips",
        "notes",
    ] {
        assert!(
            defs["ParseFailure"]["properties"][part].is_object(),
            "{part}: {schema}"
        );
    }
}

#[test]
fn envelope_schema_exists() {
    let text = serde_json::to_string(&schema_for!(Envelope<String>)).unwrap();
    assert!(
        text.contains("status") || text.contains("Envelope"),
        "{text}"
    );
}

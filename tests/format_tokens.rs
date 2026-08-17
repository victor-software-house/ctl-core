//! `pretty` / `json` token parse.
#![allow(missing_docs)]

use ctl_core::format::ParseFormatError;
use ctl_core::prelude::*;

#[test]
fn parse_known_tokens() {
    assert_eq!(
        "pretty".parse::<OutputFormat>().unwrap(),
        OutputFormat::Pretty
    );
    assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
}

#[test]
fn parse_rejects_unknown_tokens() {
    for raw in ["", "JSON", "yaml", "text", "human"] {
        assert_eq!(
            raw.parse::<OutputFormat>(),
            Err(ParseFormatError),
            "{raw:?}"
        );
    }
}

#[test]
fn json_predicate() {
    assert!(OutputFormat::Json.is_json());
    assert!(!OutputFormat::Pretty.is_json());
}

#[test]
fn display_roundtrip() {
    for mode in [OutputFormat::Pretty, OutputFormat::Json] {
        assert_eq!(mode.to_string().parse::<OutputFormat>().unwrap(), mode);
    }
}

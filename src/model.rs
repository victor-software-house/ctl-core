//! Schema-first wire types. Views render these; they do not own data.

/// Bump when the envelope shape changes.
pub const SCHEMA_VERSION: u32 = 1;

/// Hosts `JsonSchema`. schemars expands `concat!`; this module is the allow.
mod data {
    #![allow(clippy::disallowed_macros)]

    /// Machine envelope. Pretty views ignore this and render `data` / `error`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "json", serde(tag = "status", rename_all = "snake_case"))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub enum Envelope<T> {
        /// Successful payload.
        Ok {
            /// Envelope schema version.
            schema_version: u32,
            /// Command result.
            data: T,
        },
        /// Failed payload.
        Err {
            /// Envelope schema version.
            schema_version: u32,
            /// Error body.
            error: ErrorBody,
        },
    }

    /// Human and machine error payload.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub struct ErrorBody {
        /// Binary name (`qctl`, `verctl`, …).
        pub bin: String,
        /// Display message. Use `{error:#}` when the source is `anyhow`.
        pub message: String,
    }
}

pub use data::{Envelope, ErrorBody};

impl<T> Envelope<T> {
    /// Wrap `data` in a current-version success envelope.
    #[must_use]
    pub fn ok(data: T) -> Self {
        Self::Ok {
            schema_version: SCHEMA_VERSION,
            data,
        }
    }

    /// Wrap `error` in a current-version failure envelope.
    #[must_use]
    pub fn err(error: ErrorBody) -> Self {
        Self::Err {
            schema_version: SCHEMA_VERSION,
            error,
        }
    }
}

impl ErrorBody {
    /// Build an error from a binary name and message.
    #[must_use]
    pub fn new(bin: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            bin: bin.into(),
            message: message.into(),
        }
    }
}

#[cfg(all(test, feature = "json"))]
mod tests {
    use indoc::indoc;

    use super::{Envelope, ErrorBody, SCHEMA_VERSION};

    #[test]
    fn ok_envelope_roundtrip() {
        let env = Envelope::ok("demo");
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("\"schema_version\":1"));
        let back: Envelope<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Envelope::ok("demo".to_owned()));
        assert_eq!(SCHEMA_VERSION, 1);
    }

    #[test]
    fn err_envelope_shape() {
        let env = Envelope::<()>::err(ErrorBody::new("toy", "missing token"));
        let json = serde_json::to_string_pretty(&env).unwrap();
        let expected = indoc! {r#"
            {
              "status": "err",
              "schema_version": 1,
              "error": {
                "bin": "toy",
                "message": "missing token"
              }
            }
        "#};
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let expect: serde_json::Value = serde_json::from_str(expected).unwrap();
        assert_eq!(value, expect);
    }
}

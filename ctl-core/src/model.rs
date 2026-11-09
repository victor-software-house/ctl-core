//! The JSON envelope types. A view serializes a result as is. The envelope
//! wraps a success or a failure for a reader that wants a status and a schema
//! version: [`Envelope::Ok`] holds a result, and [`Envelope::Err`] holds an
//! [`ErrorBody`].
//!
//! `View::emit_err` writes an `Envelope::Err` for a failure with no model.
//! The types serialize with the `json` feature and describe themselves with
//! `schema`.
//!
//! ```
//! use ctl_core::model::{Envelope, ErrorBody, SCHEMA_VERSION};
//!
//! let failed = Envelope::<()>::err(ErrorBody::new("toy", "no such row"));
//! assert!(matches!(
//!     failed,
//!     Envelope::Err {
//!         schema_version: SCHEMA_VERSION,
//!         ..
//!     }
//! ));
//! ```

/// The envelope shape version, written to every envelope as `schema_version`.
/// ctl-core raises it when the envelope shape changes.
pub const SCHEMA_VERSION: u32 = 1;

/// Hosts `JsonSchema`. With `schema`, schemars expands `concat!`; this module
/// is the allow, and only that feature needs it.
mod data {
    #![cfg_attr(
        feature = "schema",
        expect(
            clippy::disallowed_macros,
            reason = "schemars' JsonSchema derive expands concat!"
        )
    )]

    /// A JSON envelope: `status` is `ok` with `data`, or `err` with `error`,
    /// and both carry `schema_version`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "json", serde(tag = "status", rename_all = "snake_case"))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub enum Envelope<T> {
        /// A success. Serializes with `"status": "ok"`.
        Ok {
            /// The envelope shape version,
            /// [`SCHEMA_VERSION`](super::SCHEMA_VERSION).
            schema_version: u32,
            /// The command's result.
            data: T,
        },
        /// A failure. Serializes with `"status": "err"`.
        Err {
            /// The envelope shape version,
            /// [`SCHEMA_VERSION`](super::SCHEMA_VERSION).
            schema_version: u32,
            /// What failed.
            error: ErrorBody,
        },
    }

    /// The payload of a failure envelope: which binary failed, and why.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub struct ErrorBody {
        /// The name of the binary that failed, such as `toy`.
        pub bin: String,
        /// The message to show. Format an `anyhow` error with `{error:#}` to
        /// include its whole chain.
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
    /// Build an error body from the failing binary's name and a message.
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

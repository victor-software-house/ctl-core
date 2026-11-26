//! The JSON envelope types. A view serializes a result as is. The envelope
//! wraps a success or a failure for a reader that wants a status and a schema
//! version: [`Envelope::Ok`] holds a result, and [`Envelope::Err`] holds an
//! [`ErrorBody`].
//!
//! `View::emit_err` writes an `Envelope::Err` for a failure with no model.
//! A JSON failure goes to stderr, and stdout stays empty. The types serialize
//! with the `json` feature and describe themselves with `schema`.
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
    ///
    /// Build one with [`Envelope::ok`] or [`Envelope::err`]. Each variant can
    /// gain a field in a minor release, so a pattern outside ctl-core ends
    /// with `..`.
    #[expect(
        clippy::large_enum_variant,
        reason = "a failure builds one envelope, and boxing `error` would change a public field"
    )]
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "json", serde(tag = "status", rename_all = "snake_case"))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub enum Envelope<T> {
        /// A success. Serializes with `"status": "ok"`.
        #[non_exhaustive]
        Ok {
            /// The envelope shape version,
            /// [`SCHEMA_VERSION`](super::SCHEMA_VERSION).
            schema_version: u32,
            /// The command's result.
            data: T,
        },
        /// A failure. Serializes with `"status": "err"`.
        #[non_exhaustive]
        Err {
            /// The envelope shape version,
            /// [`SCHEMA_VERSION`](super::SCHEMA_VERSION).
            schema_version: u32,
            /// What failed.
            error: ErrorBody,
        },
    }

    /// The payload of a failure envelope: which binary failed, why, and a word
    /// that names the failure when the CLI gives one.
    ///
    /// Each part of a failure has a field of its own, and JSON leaves out a
    /// field that is empty. `message` is one sentence. The causes of a handler
    /// error, the problems of a refused input, the usage line of a parse error,
    /// and clap's parts of that error go in `causes`, `problems`, `usage`, and
    /// `parse`.
    ///
    /// Build one with [`ErrorBody::new`], and add the rest with its builders,
    /// such as [`ErrorBody::kind`] and [`ErrorBody::causes`]. A field can join
    /// it in a minor release, so a struct literal outside ctl-core does not
    /// compile.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[non_exhaustive]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub struct ErrorBody {
        /// The name of the binary that failed, such as `toy`.
        pub bin: String,
        /// What failed, in one sentence. It never holds a part that has a
        /// field of its own: the further causes of an `anyhow` error are in
        /// `causes`, and the problems of an input in `problems`.
        pub message: String,
        /// A word a script can match on, such as `not_found`, which stays
        /// the same when the message is reworded. The CLI names its values.
        /// JSON leaves the field out when it is `None`, and the schema does
        /// not require it.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub kind: Option<String>,
        /// Each further error of an `anyhow` chain after `message`, outermost
        /// first. JSON leaves the field out when it is empty.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub causes: Vec<String>,
        /// The problems of a refused input, one per problem, in the order
        /// found. JSON leaves the field out when it is empty.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub problems: Vec<InputProblem>,
        /// The usage line of the command a parse error selected, without the
        /// `Usage:` label and with no styling, such as
        /// `toy show [OPTIONS] <ID>`. JSON leaves the field out when it is
        /// `None`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub usage: Option<String>,
        /// The parts of a parse error. JSON leaves the field out when it is
        /// `None`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub parse: Option<ParseFailure>,
    }

    /// One problem of a refused input, placed on its line when the text allows
    /// it.
    ///
    /// Build one with [`InputProblem::new`] and place it with its builders. A
    /// field can join it in a minor release.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[non_exhaustive]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub struct InputProblem {
        /// The input's name, usually its path.
        pub input: String,
        /// The 1-based line. JSON leaves the field out when it is `None`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub line: Option<usize>,
        /// The 1-based column. JSON leaves the field out when it is `None`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub column: Option<usize>,
        /// The field it concerns, such as `packages[1].name`. JSON leaves the
        /// field out when it is `None`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub field: Option<String>,
        /// What a person changes to fix it.
        pub message: String,
    }

    /// The parts of a parse error, as clap reports them.
    ///
    /// `reason` is always there. The rest come from clap's error context, so a
    /// build without ctl-core's `error-context` feature leaves them empty,
    /// and JSON leaves each empty field out. Build one with
    /// [`ParseFailure::new`] and add the parts with its builders. A field can
    /// join it in a minor release.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[non_exhaustive]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub struct ParseFailure {
        /// Clap's error kind in snake case, such as `unknown_argument`.
        pub reason: String,
        /// The argument the error is about, such as `--bogus`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub argument: Option<String>,
        /// The value clap refused.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub value: Option<String>,
        /// The values the argument accepts.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub valid_values: Vec<String>,
        /// The arguments, values, or subcommands clap found close to what was
        /// given.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub suggestions: Vec<String>,
        /// The arguments the argument cannot be used with.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub conflicts_with: Vec<String>,
        /// Clap's tips, such as how to pass a value that looks like a flag.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub tips: Vec<String>,
    }
}

pub use data::{Envelope, ErrorBody, InputProblem, ParseFailure};

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
    /// Build an error body from the failing binary's name and a message, with
    /// no `kind`.
    #[must_use]
    pub fn new(bin: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            bin: bin.into(),
            message: message.into(),
            kind: None,
            causes: Vec::new(),
            problems: Vec::new(),
            usage: None,
            parse: None,
        }
    }

    /// Set `kind`, the word that names the failure, such as `not_found`.
    #[must_use]
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Set `causes`, each further error after `message`, outermost first.
    #[must_use]
    pub fn causes(mut self, causes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.causes = causes.into_iter().map(Into::into).collect();
        self
    }

    /// Set `problems`, the problems of a refused input in the order found.
    #[must_use]
    pub fn problems(mut self, problems: impl IntoIterator<Item = InputProblem>) -> Self {
        self.problems = problems.into_iter().collect();
        self
    }

    /// Set `usage`, the usage line without its `Usage:` label.
    #[must_use]
    pub fn usage(mut self, usage: impl Into<String>) -> Self {
        self.usage = Some(usage.into());
        self
    }

    /// Set `parse`, the parts of a parse error.
    #[must_use]
    pub fn parse(mut self, parse: ParseFailure) -> Self {
        self.parse = Some(parse);
        self
    }
}

impl InputProblem {
    /// Build a problem of the input `input` from its message, placed on no
    /// line and no field.
    #[must_use]
    pub fn new(input: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            line: None,
            column: None,
            field: None,
            message: message.into(),
        }
    }

    /// Set `line`, 1-based.
    #[must_use]
    pub const fn line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set `column`, 1-based.
    #[must_use]
    pub const fn column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Set `field`, the field the problem concerns.
    #[must_use]
    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

impl ParseFailure {
    /// Build the parts of a parse error from its `reason`, with no other
    /// part.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            argument: None,
            value: None,
            valid_values: Vec::new(),
            suggestions: Vec::new(),
            conflicts_with: Vec::new(),
            tips: Vec::new(),
        }
    }

    /// Set `argument`, the argument the error is about.
    #[must_use]
    pub fn argument(mut self, argument: impl Into<String>) -> Self {
        self.argument = Some(argument.into());
        self
    }

    /// Set `value`, the value clap refused.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set `valid_values`, the values the argument accepts.
    #[must_use]
    pub fn valid_values(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.valid_values = values.into_iter().map(Into::into).collect();
        self
    }

    /// Set `suggestions`, what clap found close to what was given.
    #[must_use]
    pub fn suggestions(mut self, suggestions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.suggestions = suggestions.into_iter().map(Into::into).collect();
        self
    }

    /// Set `conflicts_with`, the arguments the argument cannot be used with.
    #[must_use]
    pub fn conflicts_with(
        mut self,
        arguments: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.conflicts_with = arguments.into_iter().map(Into::into).collect();
        self
    }

    /// Set `tips`, clap's tips.
    #[must_use]
    pub fn tips(mut self, tips: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tips = tips.into_iter().map(Into::into).collect();
        self
    }
}

#[cfg(all(test, feature = "json"))]
mod tests {
    use indoc::indoc;

    use super::{Envelope, ErrorBody, InputProblem, ParseFailure, SCHEMA_VERSION};

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

    #[test]
    fn a_kind_joins_the_error_and_reads_back() {
        let body = ErrorBody::new("toy", "no such box").kind("not_found");
        let json = serde_json::to_value(Envelope::<()>::err(body.clone())).unwrap();
        assert_eq!(json["error"]["kind"], "not_found");
        let back: ErrorBody = serde_json::from_value(json["error"].clone()).unwrap();
        assert_eq!(back, body);
        let without: ErrorBody =
            serde_json::from_str(r#"{"bin":"toy","message":"failed"}"#).unwrap();
        assert_eq!(without.kind, None);
    }

    #[test]
    fn each_new_field_is_left_out_when_empty() {
        let json = serde_json::to_value(ErrorBody::new("toy", "failed")).unwrap();
        assert_eq!(json, serde_json::json!({"bin": "toy", "message": "failed"}));
        let body = ErrorBody::new("toy", "failed")
            .causes(Vec::<String>::new())
            .problems([InputProblem::new("app.yaml", "bad")])
            .parse(ParseFailure::new("unknown_argument"));
        let json = serde_json::to_value(body).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "bin": "toy",
                "message": "failed",
                "problems": [{"input": "app.yaml", "message": "bad"}],
                "parse": {"reason": "unknown_argument"},
            })
        );
    }

    #[test]
    fn each_new_field_serializes_and_reads_back() {
        let body = ErrorBody::new("toy", "reading config")
            .kind("usage")
            .causes(["no such file", "permission denied"])
            .problems([InputProblem::new("app.yaml", "unknown field")
                .line(2)
                .column(1)
                .field("bogus")])
            .usage("toy show [OPTIONS] <ID>")
            .parse(
                ParseFailure::new("invalid_value")
                    .argument("--format <FORMAT>")
                    .value("xml")
                    .valid_values(["pretty", "json"])
                    .suggestions(["json"])
                    .conflicts_with(["--json"])
                    .tips(["pass it after --"]),
            );
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "bin": "toy",
                "message": "reading config",
                "kind": "usage",
                "causes": ["no such file", "permission denied"],
                "problems": [{
                    "input": "app.yaml",
                    "line": 2,
                    "column": 1,
                    "field": "bogus",
                    "message": "unknown field",
                }],
                "usage": "toy show [OPTIONS] <ID>",
                "parse": {
                    "reason": "invalid_value",
                    "argument": "--format <FORMAT>",
                    "value": "xml",
                    "valid_values": ["pretty", "json"],
                    "suggestions": ["json"],
                    "conflicts_with": ["--json"],
                    "tips": ["pass it after --"],
                },
            })
        );
        assert_eq!(serde_json::from_value::<ErrorBody>(json).unwrap(), body);
    }
}

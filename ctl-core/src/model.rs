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
        /// What a person reads in the non-JSON error: the text the failure
        /// printed before it had fields, such as clap's whole message with its
        /// tips and usage, or an `anyhow` chain joined with `: `. JSON carries
        /// the parts in their own fields, so this stays out of JSON and out of
        /// the schema. `None` prints `message`.
        #[cfg_attr(feature = "json", serde(skip))]
        #[cfg_attr(feature = "schema", schemars(skip))]
        pub(super) text: Option<String>,
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

    /// The parts of a parse error, one field per kind of context clap gives.
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
        /// The argument the error is about, such as `--bogus`: clap's
        /// `InvalidArg`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub argument: Option<String>,
        /// The required arguments the command line left out, such as `<TO>`:
        /// clap's `InvalidArg` of a missing required argument.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub missing: Vec<String>,
        /// The value clap refused: `InvalidValue`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub value: Option<String>,
        /// The values the argument accepts: `ValidValue`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub valid_values: Vec<String>,
        /// The subcommand the error is about: the one clap refused, or the
        /// command that needs a subcommand it was not given. Clap's
        /// `InvalidSubcommand`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub subcommand: Option<String>,
        /// The subcommands the command accepts: `ValidSubcommand`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub valid_subcommands: Vec<String>,
        /// The arguments clap found close to the one given: `SuggestedArg`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub suggested_arguments: Vec<String>,
        /// The values clap found close to the one given: `SuggestedValue`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub suggested_values: Vec<String>,
        /// The subcommands clap found close to the one given:
        /// `SuggestedSubcommand`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub suggested_subcommands: Vec<String>,
        /// The commands clap suggests instead: `SuggestedCommand`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub suggested_commands: Vec<String>,
        /// The arguments the argument cannot be used with: `PriorArg`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub conflicts_with: Vec<String>,
        /// How many values the argument got: `ActualNumValues`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub actual_values: Option<usize>,
        /// How many values the argument takes: `ExpectedNumValues`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub expected_values: Option<usize>,
        /// The fewest values the argument takes: `MinValues`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub min_values: Option<usize>,
        /// The argument that follows `--`: `TrailingArg`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        pub trailing_argument: Option<String>,
        /// Clap's tips, such as how to pass a value that looks like a flag:
        /// `Suggested`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub tips: Vec<String>,
        /// Messages a CLI's own code attached to the error: `Custom`.
        #[cfg_attr(
            feature = "json",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub notes: Vec<String>,
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
            text: None,
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

    /// Set the text a person reads, which `pretty` and the other
    /// non-JSON formats write after `{bin}: ` in place of `message`.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// The text a person reads, when the failure has one that `message`
    /// does not give: what it printed before `message` was one sentence.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
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
            missing: Vec::new(),
            value: None,
            valid_values: Vec::new(),
            subcommand: None,
            valid_subcommands: Vec::new(),
            suggested_arguments: Vec::new(),
            suggested_values: Vec::new(),
            suggested_subcommands: Vec::new(),
            suggested_commands: Vec::new(),
            conflicts_with: Vec::new(),
            actual_values: None,
            expected_values: None,
            min_values: None,
            trailing_argument: None,
            tips: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Set `argument`, the argument the error is about, such as `--bogus`:
    /// clap's `InvalidArg`.
    #[must_use]
    pub fn argument(mut self, argument: impl Into<String>) -> Self {
        self.argument = Some(argument.into());
        self
    }

    /// Set `missing`, the required arguments the command line left out, such as
    /// `<TO>`: clap's `InvalidArg` of a missing required argument.
    #[must_use]
    pub fn missing(mut self, missing: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.missing = missing.into_iter().map(Into::into).collect();
        self
    }

    /// Set `value`, the value clap refused: `InvalidValue`.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set `valid_values`, the values the argument accepts: `ValidValue`.
    #[must_use]
    pub fn valid_values(
        mut self,
        valid_values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.valid_values = valid_values.into_iter().map(Into::into).collect();
        self
    }

    /// Set `subcommand`, the subcommand the error is about: the one clap
    /// refused, or the command that needs a subcommand it was not given. Clap's
    /// `InvalidSubcommand`.
    #[must_use]
    pub fn subcommand(mut self, subcommand: impl Into<String>) -> Self {
        self.subcommand = Some(subcommand.into());
        self
    }

    /// Set `valid_subcommands`, the subcommands the command accepts:
    /// `ValidSubcommand`.
    #[must_use]
    pub fn valid_subcommands(
        mut self,
        valid_subcommands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.valid_subcommands = valid_subcommands.into_iter().map(Into::into).collect();
        self
    }

    /// Set `suggested_arguments`, the arguments clap found close to the one
    /// given: `SuggestedArg`.
    #[must_use]
    pub fn suggested_arguments(
        mut self,
        suggested_arguments: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.suggested_arguments = suggested_arguments.into_iter().map(Into::into).collect();
        self
    }

    /// Set `suggested_values`, the values clap found close to the one given:
    /// `SuggestedValue`.
    #[must_use]
    pub fn suggested_values(
        mut self,
        suggested_values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.suggested_values = suggested_values.into_iter().map(Into::into).collect();
        self
    }

    /// Set `suggested_subcommands`, the subcommands clap found close to the one
    /// given: `SuggestedSubcommand`.
    #[must_use]
    pub fn suggested_subcommands(
        mut self,
        suggested_subcommands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.suggested_subcommands = suggested_subcommands.into_iter().map(Into::into).collect();
        self
    }

    /// Set `suggested_commands`, the commands clap suggests instead:
    /// `SuggestedCommand`.
    #[must_use]
    pub fn suggested_commands(
        mut self,
        suggested_commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.suggested_commands = suggested_commands.into_iter().map(Into::into).collect();
        self
    }

    /// Set `conflicts_with`, the arguments the argument cannot be used with:
    /// `PriorArg`.
    #[must_use]
    pub fn conflicts_with(
        mut self,
        conflicts_with: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.conflicts_with = conflicts_with.into_iter().map(Into::into).collect();
        self
    }

    /// Set `actual_values`, how many values the argument got:
    /// `ActualNumValues`.
    #[must_use]
    pub const fn actual_values(mut self, actual_values: usize) -> Self {
        self.actual_values = Some(actual_values);
        self
    }

    /// Set `expected_values`, how many values the argument takes:
    /// `ExpectedNumValues`.
    #[must_use]
    pub const fn expected_values(mut self, expected_values: usize) -> Self {
        self.expected_values = Some(expected_values);
        self
    }

    /// Set `min_values`, the fewest values the argument takes: `MinValues`.
    #[must_use]
    pub const fn min_values(mut self, min_values: usize) -> Self {
        self.min_values = Some(min_values);
        self
    }

    /// Set `trailing_argument`, the argument that follows `--`: `TrailingArg`.
    #[must_use]
    pub fn trailing_argument(mut self, trailing_argument: impl Into<String>) -> Self {
        self.trailing_argument = Some(trailing_argument.into());
        self
    }

    /// Set `tips`, clap's tips, such as how to pass a value that looks like a
    /// flag: `Suggested`.
    #[must_use]
    pub fn tips(mut self, tips: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tips = tips.into_iter().map(Into::into).collect();
        self
    }

    /// Set `notes`, messages a CLI's own code attached to the error: `Custom`.
    #[must_use]
    pub fn notes(mut self, notes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.notes = notes.into_iter().map(Into::into).collect();
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
                    .suggested_values(["json"])
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
                    "suggested_values": ["json"],
                    "conflicts_with": ["--json"],
                    "tips": ["pass it after --"],
                },
            })
        );
        assert_eq!(serde_json::from_value::<ErrorBody>(json).unwrap(), body);
    }

    #[test]
    fn the_text_a_person_reads_stays_out_of_json() {
        let body = ErrorBody::new("toy", "failed").with_text("failed\n\nUsage: toy");
        assert_eq!(body.text(), Some("failed\n\nUsage: toy"));
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json, serde_json::json!({"bin": "toy", "message": "failed"}));
        assert_eq!(
            serde_json::from_value::<ErrorBody>(json).unwrap().text(),
            None
        );
    }
}

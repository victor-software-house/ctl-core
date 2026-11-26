//! The [`ErrorBody`] of a parse error and of a handler error: one field per
//! part, so a caller that reads the JSON envelope never parses prose.

use clap::error::ErrorKind;
#[cfg(feature = "error-context")]
use clap::error::{ContextKind, ContextValue};
use clap::{ArgMatches, Command};

use crate::help;
#[cfg(any(feature = "input-yaml", feature = "input-json", feature = "input-toml"))]
use crate::input::{InputError, Problem};
#[cfg(any(feature = "input-yaml", feature = "input-json", feature = "input-toml"))]
use crate::model::InputProblem;
use crate::model::{ErrorBody, ParseFailure};

/// The body of a parse error: clap's first line as `message`, clap's whole
/// text as `text` for the formats that print it, the reason and, with
/// `error-context`, clap's parts as `parse`, and the usage line of the
/// command `selected` read from.
///
/// `selected` is what the lenient read of the arguments found, so the usage
/// line is the one of the subcommand the caller reached, or the root's when
/// the read found none.
pub(super) fn parse_error(
    bin: &str,
    error: &clap::Error,
    command: &Command,
    selected: Option<&ArgMatches>,
) -> ErrorBody {
    let rendered = error.to_string();
    let text = parse_error_text(&rendered);
    ErrorBody::new(bin, text.lines().next().unwrap_or_default().to_owned())
        .with_text(text.to_owned())
        .usage(help::usage_line(command, selected))
        .parse(parse_failure(error))
}

/// Clap's rendered error without its leading `error: `, so a parse error
/// reads `bin: text` like every other error.
pub(super) fn parse_error_text(rendered: &str) -> &str {
    rendered.trim().trim_prefix("error: ")
}

/// The body of an error a handler returned: its outermost error as
/// `message` and the rest of its chain as `causes`.
///
/// `text` is the whole chain joined with `: `, as `{error:#}` prints it.
///
/// With an input feature, an [`InputError`] in the chain fills `problems`
/// and is no cause. One that is the whole error gives `message` its summary.
pub(super) fn handler_error(bin: &str, error: &anyhow::Error) -> ErrorBody {
    let mut links = error.chain().map(ToString::to_string).collect::<Vec<_>>();
    let text = links.join(": ");
    let message = links.remove(0);
    #[cfg(any(feature = "input-yaml", feature = "input-json", feature = "input-toml"))]
    {
        let refused = error
            .chain()
            .filter_map(|cause| cause.downcast_ref::<InputError>())
            .collect::<Vec<_>>();
        if !refused.is_empty() {
            let message = error
                .chain()
                .next()
                .and_then(|outermost| outermost.downcast_ref::<InputError>())
                .map_or(message, summary);
            let causes = error
                .chain()
                .skip(1)
                .zip(links)
                .filter(|(cause, _)| !cause.is::<InputError>())
                .map(|(_, link)| link);
            let problems = refused.iter().flat_map(|refused| {
                refused
                    .problems
                    .iter()
                    .map(|problem| input_problem(&refused.name, problem))
            });
            return ErrorBody::new(bin, message)
                .causes(causes)
                .problems(problems)
                .with_text(text);
        }
    }
    ErrorBody::new(bin, message).causes(links).with_text(text)
}

/// The body of an input document ctl-core refused: its summary as `message`
/// and each problem in `problems`, as for any refused input, so the pretty
/// text is the `InputError`'s own display, one problem per line.
#[cfg(feature = "input-document")]
pub(super) fn refused_document(bin: &str, refused: InputError) -> ErrorBody {
    handler_error(bin, &anyhow::Error::new(refused))
}

/// The body of a clap error that `refused` places on a key of the document:
/// the one problem, and clap's parts as `parse` and the usage line of the
/// command `selected` read from, as for any parse error.
#[cfg(feature = "input-document")]
pub(super) fn refused_value(
    bin: &str,
    refused: InputError,
    error: &clap::Error,
    command: &Command,
    selected: Option<&ArgMatches>,
) -> ErrorBody {
    refused_document(bin, refused)
        .usage(help::usage_line(command, selected))
        .parse(parse_failure(error))
}

/// What a refused input is in one sentence: its name and the number of its
/// problems, whatever the number. The problems themselves are `problems`.
#[cfg(any(feature = "input-yaml", feature = "input-json", feature = "input-toml"))]
fn summary(refused: &InputError) -> String {
    let count = refused.problems.len();
    let noun = if count == 1 { "problem" } else { "problems" };
    format!("{} has {count} {noun}", refused.name)
}

#[cfg(any(feature = "input-yaml", feature = "input-json", feature = "input-toml"))]
fn input_problem(input: &str, problem: &Problem) -> InputProblem {
    let mut placed = InputProblem::new(input, problem.message.as_str());
    if let Some(line) = problem.line {
        placed = placed.line(line);
    }
    if let Some(column) = problem.column {
        placed = placed.column(column);
    }
    if !problem.field.is_empty() {
        placed = placed.field(problem.field.as_str());
    }
    placed
}

/// The reason of a parse error, and with `error-context` the parts clap read.
fn parse_failure(error: &clap::Error) -> ParseFailure {
    let parse = ParseFailure::new(reason(error.kind()));
    #[cfg(feature = "error-context")]
    let parse = context(error, parse);
    parse
}

/// Clap's error kind in snake case. A kind a later clap adds reads as
/// `unknown`, since the set is not closed.
const fn reason(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::InvalidValue => "invalid_value",
        ErrorKind::UnknownArgument => "unknown_argument",
        ErrorKind::InvalidSubcommand => "invalid_subcommand",
        ErrorKind::NoEquals => "no_equals",
        ErrorKind::ValueValidation => "value_validation",
        ErrorKind::TooManyValues => "too_many_values",
        ErrorKind::TooFewValues => "too_few_values",
        ErrorKind::WrongNumberOfValues => "wrong_number_of_values",
        ErrorKind::ArgumentConflict => "argument_conflict",
        ErrorKind::MissingRequiredArgument => "missing_required_argument",
        ErrorKind::MissingSubcommand => "missing_subcommand",
        ErrorKind::InvalidUtf8 => "invalid_utf8",
        ErrorKind::DisplayHelp => "display_help",
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            "display_help_on_missing_argument_or_subcommand"
        }
        ErrorKind::DisplayVersion => "display_version",
        ErrorKind::Io => "io",
        ErrorKind::Format => "format",
        _ => "unknown",
    }
}

/// `parse` with the parts clap's error context holds, each in the field of its
/// kind. A `Usage` is left out, since ctl-core renders the usage line itself
/// for every parse error, and so is a kind a later clap adds.
#[cfg(feature = "error-context")]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "ContextKind is non_exhaustive, and Usage has no field"
)]
fn context(error: &clap::Error, mut parse: ParseFailure) -> ParseFailure {
    let missing = error.kind() == ErrorKind::MissingRequiredArgument;
    for (kind, value) in error.context() {
        let texts = plain(value);
        let one = || (!texts.is_empty()).then(|| texts.join(", "));
        match kind {
            ContextKind::InvalidArg if missing => parse.missing = texts,
            ContextKind::InvalidArg => parse.argument = one(),
            ContextKind::InvalidValue => parse.value = one(),
            ContextKind::InvalidSubcommand => parse.subcommand = one(),
            ContextKind::TrailingArg => parse.trailing_argument = one(),
            ContextKind::ValidValue => parse.valid_values = texts,
            ContextKind::ValidSubcommand => parse.valid_subcommands = texts,
            ContextKind::SuggestedArg => parse.suggested_arguments = texts,
            ContextKind::SuggestedValue => parse.suggested_values = texts,
            ContextKind::SuggestedSubcommand => parse.suggested_subcommands = texts,
            ContextKind::SuggestedCommand => parse.suggested_commands = texts,
            ContextKind::PriorArg => parse.conflicts_with = texts,
            ContextKind::Suggested => parse.tips = texts,
            ContextKind::Custom => parse.notes = texts,
            ContextKind::ActualNumValues => parse.actual_values = count(value),
            ContextKind::ExpectedNumValues => parse.expected_values = count(value),
            ContextKind::MinValues => parse.min_values = count(value),
            _ => {}
        }
    }
    parse
}

/// The number of a context value.
#[cfg(feature = "error-context")]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "ContextValue is non_exhaustive, and only Number is a count"
)]
fn count(value: &ContextValue) -> Option<usize> {
    match value {
        ContextValue::Number(number) => usize::try_from(*number).ok(),
        _ => None,
    }
}

/// The strings of one context value, with no styling.
#[cfg(feature = "error-context")]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "ContextValue is non_exhaustive, and a value a later clap adds reads as its text"
)]
fn plain(value: &ContextValue) -> Vec<String> {
    match value {
        ContextValue::None => Vec::new(),
        ContextValue::String(text) => vec![text.clone()],
        ContextValue::Strings(texts) => texts.clone(),
        ContextValue::StyledStr(text) => vec![text.to_string()],
        ContextValue::StyledStrs(texts) => texts.iter().map(ToString::to_string).collect(),
        other => vec![other.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::{parse_error_text, reason};

    #[test]
    fn a_parse_error_text_is_claps_message_without_its_prefix() {
        assert_eq!(
            parse_error_text("error: unexpected argument found\n"),
            "unexpected argument found"
        );
        assert_eq!(parse_error_text("no prefix\n"), "no prefix");
        assert_eq!(
            parse_error_text(
                "error: unexpected argument '--bogus' found\n\n  tip: pass it after --\n\nUsage: toy\n"
            ),
            "unexpected argument '--bogus' found\n\n  tip: pass it after --\n\nUsage: toy"
        );
    }

    #[test]
    fn a_reason_is_claps_kind_in_snake_case() {
        assert_eq!(reason(ErrorKind::UnknownArgument), "unknown_argument");
        assert_eq!(
            reason(ErrorKind::MissingRequiredArgument),
            "missing_required_argument"
        );
    }

    /// Every `ContextKind` of `clap_builder` 4.6.6, with the `parse` field that
    /// carries it, or the reason it has none. A kind a later clap adds needs
    /// a row here and a field, or a reason.
    #[cfg(feature = "error-context")]
    mod context {
        use clap::error::{ContextKind, ContextValue, ErrorKind};

        use super::super::parse_failure;

        enum Home {
            Field(&'static str),
            Without(&'static str),
        }

        const KINDS: [(ContextKind, Home); 17] = [
            (ContextKind::InvalidSubcommand, Home::Field("subcommand")),
            (ContextKind::InvalidArg, Home::Field("argument")),
            (ContextKind::PriorArg, Home::Field("conflicts_with")),
            (
                ContextKind::ValidSubcommand,
                Home::Field("valid_subcommands"),
            ),
            (ContextKind::ValidValue, Home::Field("valid_values")),
            (ContextKind::InvalidValue, Home::Field("value")),
            (ContextKind::ActualNumValues, Home::Field("actual_values")),
            (
                ContextKind::ExpectedNumValues,
                Home::Field("expected_values"),
            ),
            (ContextKind::MinValues, Home::Field("min_values")),
            (
                ContextKind::SuggestedCommand,
                Home::Field("suggested_commands"),
            ),
            (
                ContextKind::SuggestedSubcommand,
                Home::Field("suggested_subcommands"),
            ),
            (
                ContextKind::SuggestedArg,
                Home::Field("suggested_arguments"),
            ),
            (ContextKind::SuggestedValue, Home::Field("suggested_values")),
            (ContextKind::TrailingArg, Home::Field("trailing_argument")),
            (ContextKind::Suggested, Home::Field("tips")),
            (
                ContextKind::Usage,
                Home::Without("ctl-core renders the usage line itself, as ErrorBody::usage"),
            ),
            (ContextKind::Custom, Home::Field("notes")),
        ];

        #[test]
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "only the three counts take a number, and every other kind a list"
        )]
        fn every_kind_of_clap_context_has_a_field_or_a_reason() {
            for (kind, home) in KINDS {
                let value = match kind {
                    ContextKind::ActualNumValues
                    | ContextKind::ExpectedNumValues
                    | ContextKind::MinValues => ContextValue::Number(3),
                    _ => ContextValue::Strings(vec!["one".to_owned(), "two".to_owned()]),
                };
                let mut error = clap::Error::new(ErrorKind::UnknownArgument);
                error.insert(kind, value);
                let parse = serde_json::to_value(parse_failure(&error)).unwrap();
                match home {
                    Home::Field(field) => assert!(
                        parse.get(field).is_some_and(|part| !part.is_null()),
                        "{kind:?} has no {field}: {parse}"
                    ),
                    Home::Without(reason) => assert_eq!(
                        parse,
                        serde_json::json!({"reason": "unknown_argument"}),
                        "{kind:?}: {reason}"
                    ),
                }
            }
        }

        #[test]
        fn a_missing_required_argument_lists_the_arguments_left_out() {
            let mut error = clap::Error::new(ErrorKind::MissingRequiredArgument);
            error.insert(
                ContextKind::InvalidArg,
                ContextValue::Strings(vec!["<TO>".to_owned()]),
            );
            let parse = parse_failure(&error);
            assert_eq!(parse.missing, ["<TO>"]);
            assert_eq!(parse.argument, None);
        }
    }
}

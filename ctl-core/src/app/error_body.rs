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

/// The body of a parse error: clap's first line as `message`, the reason
/// and, with `error-context`, clap's parts as `parse`, and the usage line of
/// the command `selected` read from.
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
    let parse = parse_failure(error);
    ErrorBody::new(bin, parse_error_message(&rendered))
        .usage(help::usage_line(command, selected))
        .parse(parse)
}

/// Clap's rendered error without its leading `error: `, and without the tip,
/// usage and help paragraphs after its first line, so a parse error reads
/// `bin: message` like every other error.
pub(super) fn parse_error_message(rendered: &str) -> &str {
    let first = rendered.trim().lines().next().unwrap_or_default();
    first.trim_prefix("error: ")
}

/// The body of an error a handler returned: its outermost error as
/// `message` and the rest of its chain as `causes`.
///
/// With an input feature, an [`InputError`] in the chain fills `problems`
/// and is no cause. One that is the whole error gives `message` its summary.
pub(super) fn handler_error(bin: &str, error: &anyhow::Error) -> ErrorBody {
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
                .map_or_else(|| error.to_string(), summary);
            let causes = error
                .chain()
                .skip(1)
                .filter(|cause| !cause.is::<InputError>())
                .map(ToString::to_string);
            let problems = refused.iter().flat_map(|refused| {
                refused
                    .problems
                    .iter()
                    .map(|problem| input_problem(&refused.name, problem))
            });
            return ErrorBody::new(bin, message)
                .causes(causes)
                .problems(problems);
        }
    }
    ErrorBody::new(bin, error.to_string()).causes(error.chain().skip(1).map(ToString::to_string))
}

/// What a refused input is in one sentence: its one problem's message, or the
/// number of its problems.
#[cfg(any(feature = "input-yaml", feature = "input-json", feature = "input-toml"))]
fn summary(refused: &InputError) -> String {
    match refused.problems.as_slice() {
        [problem] => problem.message.clone(),
        problems => format!("{} has {} problems", refused.name, problems.len()),
    }
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

/// `parse` with the parts clap's error context holds, as plain strings.
#[cfg(feature = "error-context")]
fn context(error: &clap::Error, mut parse: ParseFailure) -> ParseFailure {
    let strings = |kinds: &[ContextKind]| {
        kinds
            .iter()
            .filter_map(|kind| error.get(*kind))
            .flat_map(plain)
            .collect::<Vec<_>>()
    };
    let single = |kind: ContextKind| {
        let parts = strings(&[kind]);
        (!parts.is_empty()).then(|| parts.join(", "))
    };
    if let Some(argument) = single(ContextKind::InvalidArg) {
        parse = parse.argument(argument);
    }
    if let Some(value) = single(ContextKind::InvalidValue) {
        parse = parse.value(value);
    }
    parse
        .valid_values(strings(&[ContextKind::ValidValue]))
        .suggestions(strings(&[
            ContextKind::SuggestedArg,
            ContextKind::SuggestedValue,
            ContextKind::SuggestedSubcommand,
            ContextKind::SuggestedCommand,
        ]))
        .conflicts_with(strings(&[ContextKind::PriorArg]))
        .tips(strings(&[ContextKind::Suggested]))
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

    use super::{parse_error_message, reason};

    #[test]
    fn a_parse_error_message_is_claps_first_line_without_its_prefix() {
        assert_eq!(
            parse_error_message("error: unexpected argument found\n"),
            "unexpected argument found"
        );
        assert_eq!(parse_error_message("no prefix\n"), "no prefix");
        assert_eq!(
            parse_error_message(
                "error: unexpected argument '--bogus' found\n\n  tip: pass it after --\n\nUsage: toy\n"
            ),
            "unexpected argument '--bogus' found"
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
}

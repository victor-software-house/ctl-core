//! The YAML half of [`Input`]. It parses with serde-saphyr under the limits in
//! force, and words each problem without serde-saphyr's own position. It
//! places each garde problem through yamled's location index.

use std::borrow::Cow;

use serde::de::DeserializeOwned;
use serde_saphyr::{
    Budget, DefaultMessageFormatter, DuplicateKeyPolicy, Error, Localizer, Location,
    MergeKeyPolicy, MessageFormatter,
};

use super::limits::{Exceeded, Raise};
use super::{Input, InputError, Problem};
#[cfg(feature = "validate")]
use super::{Step, field_steps};

/// [`Input::parse`] for a YAML text, once the text's size is known to fit.
pub(super) fn parse<T: DeserializeOwned>(
    input: &Input,
    budget: &Budget,
    raise: Raise,
) -> Result<T, InputError> {
    let options = serde_saphyr::options! {
        budget: Some(budget.clone()),
        strict_booleans: true,
        duplicate_keys: DuplicateKeyPolicy::Error,
        merge_keys: MergeKeyPolicy::Error,
        with_snippet: false,
    };
    serde_saphyr::from_str_with_options(&input.text, options).map_err(|error| {
        let location = error.location();
        InputError {
            name: input.name.clone(),
            problems: vec![Problem {
                line: location
                    .and_then(|at| usize::try_from(at.line()).ok())
                    .map(|line| line + input.lines_before),
                column: location.and_then(|at| usize::try_from(at.column()).ok()),
                field: String::new(),
                message: message(input, error.without_snippet(), budget, raise),
            }],
        }
    })
}

/// serde-saphyr's wording for `error`.
///
/// An alias error also names where its value is defined, counted from the
/// file like [`Problem::line`]. A limit error names the limit in `budget`
/// and where to raise it.
///
/// For an alias error the wording and position are those of the error the
/// aliased value raised, found under any alias reached through another
/// alias. Without a known position, the anchor's position is used.
fn message(input: &Input, error: &Error, budget: &Budget, raise: Raise) -> String {
    if let Error::Budget { breach, .. } = error
        && let Some(exceeded) = Exceeded::breach(breach, budget, raise)
    {
        return exceeded.to_string();
    }
    let formatter = DefaultMessageFormatter.with_localizer(&Unplaced);
    let Error::AliasError {
        error: inner,
        locations,
        ..
    } = error
    else {
        return plain(&formatter, error);
    };
    let mut failing = inner.as_ref();
    while let Error::AliasError { error, .. } | Error::WithSnippet { error, .. } = failing {
        failing = error;
    }
    let at = failing
        .location()
        .filter(|at| *at != Location::UNKNOWN)
        .unwrap_or(locations.defined_location);
    let line = usize::try_from(at.line()).unwrap_or_default() + input.lines_before;
    let column = at.column();
    let text = plain(&formatter, failing);
    format!("{text} (defined at line {line}, column {column})")
}

/// serde-saphyr's wording for `error`, with each control character escaped
/// as serde-saphyr's renderer escapes it.
///
/// It takes the text from [`MessageFormatter::format_message`] and never calls
/// the renderer, whose snippet code would otherwise stay in every binary that
/// loads input. The loader turns snippets off, so the renderer writes the same
/// line ([vsh-serde-saphyr 0.0.1][plain]).
///
/// [plain]: https://docs.rs/crate/vsh-serde-saphyr/0.0.2/source/src/de/error.rs#1970
fn plain(formatter: &dyn MessageFormatter, mut error: &Error) -> String {
    while let Error::AliasError { error: inner, .. } | Error::WithSnippet { error: inner, .. } =
        error
    {
        error = inner;
    }
    let mut text = String::new();
    for character in formatter.format_message(error).chars() {
        if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') {
            text.extend(character.escape_debug());
        } else {
            text.push(character);
        }
    }
    text
}

/// serde-saphyr's wording without the position it appends. That position
/// counts from the parsed text, which for frontmatter is not the file, and
/// [`Problem`] already carries the file's line and column.
struct Unplaced;

impl Localizer for Unplaced {
    fn attach_location<'a>(&self, base: Cow<'a, str>, _: Location) -> Cow<'a, str> {
        base
    }

    fn alias_used_at(&self, _: Location) -> String {
        String::new()
    }
}

/// The 1-based line and column of each garde field in `text`, or of its
/// nearest written ancestor. `None` for every field when `text` does not
/// parse.
#[cfg(feature = "validate")]
pub(super) fn places(text: &str, fields: &[&str]) -> Vec<Option<(usize, usize)>> {
    let document = yamled::Document::parse(text).ok();
    fields
        .iter()
        .map(|field| {
            document
                .as_ref()
                .and_then(|document| document.locate_nearest(&path(&field_steps(field))))
                .map(|at| (at.line, at.column))
        })
        .collect()
}

/// `steps` as a yamled path.
#[cfg(feature = "validate")]
fn path(steps: &[Step<'_>]) -> yamled::Path {
    steps
        .iter()
        .fold(yamled::Path::root(), |path, step| match step {
            Step::Key(key) => path.key(*key),
            Step::Index(index) => path.index(*index),
        })
}

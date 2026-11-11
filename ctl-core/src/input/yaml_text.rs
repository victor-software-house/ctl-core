//! The YAML half of [`Input`]. It parses with serde-saphyr under the limits in
//! force, and words each problem without serde-saphyr's own position. It
//! places each problem through yamled's location index.

use std::borrow::Cow;
use std::sync::OnceLock;

use serde::de::{DeserializeOwned, IgnoredAny};
use serde_saphyr::{
    Budget, DefaultMessageFormatter, DuplicateKeyPolicy, Error, Localizer, Location,
    MergeKeyPolicy, MessageFormatter, Options,
};

use super::limits::{Exceeded, Limits, Raise};
use super::{FieldPath, Input, InputError, Problem, Step};

/// [`Input::parse`] for a YAML text, once the text's size is known to fit.
pub(super) fn parse<T: DeserializeOwned>(
    input: &Input,
    budget: &Budget,
    raise: Raise,
) -> Result<T, InputError> {
    serde_saphyr::from_str_with_options(&input.text, options(budget)).map_err(|error| {
        let location = error.location();
        InputError::of(
            input.name.clone(),
            vec![Problem {
                line: location
                    .and_then(|at| usize::try_from(at.line()).ok())
                    .map(|line| line + input.lines_before),
                column: location.and_then(|at| usize::try_from(at.column()).ok()),
                field: String::new(),
                message: message(input, error.without_snippet(), budget, raise),
            }],
        )
    })
}

/// What every read of a YAML text uses, under `budget`.
fn options(budget: &Budget) -> Options {
    serde_saphyr::options! {
        budget: Some(budget.clone()),
        strict_booleans: true,
        duplicate_keys: DuplicateKeyPolicy::Error,
        merge_keys: MergeKeyPolicy::Error,
        with_snippet: false,
    }
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

/// The yamled document of one input, built when it is first placed on and kept
/// after that.
///
/// It is derived from the text, so a clone starts with the same contents,
/// `Debug` shows them, and two inputs with the same fields are equal whether
/// or not either has been placed on.
#[derive(Clone, Debug, Default)]
pub(super) struct Located(OnceLock<Option<yamled::Document>>);

impl PartialEq for Located {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Located {}

/// The 1-based line and column, in the text, of each path or of its nearest
/// written ancestor. `None` for every path when the text does not parse or
/// goes past `limits`.
///
/// The document is built by the first call on `input` and read by every call
/// after it. A text `limits` refuse is never handed to yamled, whose parse has
/// no budget of its own, and the refusal is kept like a document would be.
pub(super) fn places(
    input: &Input,
    limits: &Limits,
    paths: &[FieldPath],
) -> Vec<Option<(usize, usize)>> {
    let document = input
        .yaml_document
        .0
        .get_or_init(|| {
            within(input, &limits.budget())
                .then(|| yamled::Document::parse(&input.text).ok())
                .flatten()
        })
        .as_ref();
    paths
        .iter()
        .map(|path| {
            document
                .and_then(|document| document.locate_nearest(&yamled_path(path)))
                .map(|at| (at.line, at.column))
        })
        .collect()
}

/// Whether serde-saphyr reads the whole text under `budget`, depth and node
/// count included, as [`parse`] would, without building any value.
fn within(input: &Input, budget: &Budget) -> bool {
    serde_saphyr::from_str_with_options::<IgnoredAny>(&input.text, options(budget)).is_ok()
}

/// `path` as a yamled path.
fn yamled_path(path: &FieldPath) -> yamled::Path {
    path.steps()
        .iter()
        .fold(yamled::Path::root(), |path, step| match step {
            Step::Key(key) => path.key(key.as_str()),
            Step::Index(index) => path.index(*index),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_placing_reuses_the_document() {
        let input = Input::new("a.yaml", "files:\n  'a.b':\n    label: ''\n");
        let path = FieldPath::root().key("files").key("a.b").key("label");
        assert!(input.yaml_document.0.get().is_none());
        assert_eq!(input.place(&path).map(|at| at.line), Some(3));
        let first = input.yaml_document.0.get().unwrap();
        assert!(first.is_some());
        assert_eq!(input.place(&path).map(|at| at.line), Some(3));
        let second = input.yaml_document.0.get().unwrap();
        assert!(std::ptr::eq(first, second));
    }

    /// `text` with the call tier's `limits`, placed at its first key.
    fn placed_with(text: &str, limits: Limits) -> (Input, Option<(usize, usize)>) {
        let input = Input::new("a.yaml", text).limits(limits);
        let at = input
            .place(&FieldPath::root().key("a"))
            .map(|at| (at.line, at.column));
        (input, at)
    }

    #[test]
    fn a_text_over_the_size_limit_builds_no_document() {
        let limits = Limits {
            max_reader_input_bytes: Some(4),
            ..Limits::NONE
        };
        let (input, at) = placed_with("a: 1\nb: 2\n", limits);
        assert_eq!(at, None);
        assert!(input.yaml_document.0.get().is_none());
    }

    #[test]
    fn a_text_over_the_depth_or_node_limit_keeps_no_document() {
        let text = "a: {b: {c: {d: 1}}}\n";
        assert_eq!(placed_with(text, Limits::NONE).1, Some((1, 4)));
        for limits in [
            Limits {
                max_depth: Some(2),
                ..Limits::NONE
            },
            Limits {
                max_nodes: Some(3),
                ..Limits::NONE
            },
        ] {
            let (input, at) = placed_with(text, limits);
            assert_eq!(at, None);
            assert!(matches!(input.yaml_document.0.get(), Some(None)));
        }
    }

    #[test]
    fn an_input_equals_its_clone_whether_or_not_it_was_placed_on() {
        let input = Input::new("a.yaml", "a: 1\n");
        let fresh = input.clone();
        assert!(input.place(&FieldPath::root().key("a")).is_some());
        assert_eq!(input, fresh);
    }
}

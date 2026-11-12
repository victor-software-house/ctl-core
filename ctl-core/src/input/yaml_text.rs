//! The YAML half of [`Input`]. It parses with serde-saphyr under the limits in
//! force, and words each problem without serde-saphyr's own position. It
//! places each problem through yamled's location index.

use std::borrow::Cow;
use std::sync::{Mutex, OnceLock, PoisonError};

use serde::de::{DeserializeOwned, IgnoredAny};
use serde_saphyr::{
    Budget, DefaultMessageFormatter, DuplicateKeyPolicy, Error, Localizer, Location,
    MergeKeyPolicy, MessageFormatter, NonFiniteFloatPolicy, Options,
};

use super::limits::{Exceeded, Limits, Raise};
use super::{At, FieldPath, Input, InputError, Problem, Step};

/// [`Input::parse`] for a YAML text, once the text's size is known to fit.
pub(super) fn parse<T: DeserializeOwned>(
    input: &Input,
    budget: &Budget,
    raise: Raise,
) -> Result<T, InputError> {
    read(
        input,
        options(budget, NonFiniteFloatPolicy::Default),
        budget,
        raise,
    )
}

/// [`parse`] for a read that only sorts each value by its type, as
/// [`Input::top_level`] does, so a `.inf` or a `.NaN` is a number and no
/// refusal.
pub(super) fn parse_types<T: DeserializeOwned>(
    input: &Input,
    budget: &Budget,
    raise: Raise,
) -> Result<T, InputError> {
    read(
        input,
        options(budget, NonFiniteFloatPolicy::PassThrough),
        budget,
        raise,
    )
}

/// Reads the text of `input` under `options`, and words each problem.
fn read<T: DeserializeOwned>(
    input: &Input,
    options: Options,
    budget: &Budget,
    raise: Raise,
) -> Result<T, InputError> {
    serde_saphyr::from_str_with_options(&input.text, options).map_err(|error| {
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
///
/// `floats` says what a `.inf`, `-.inf` or `.NaN` becomes when the caller names
/// no type for it: `Default` refuses it, and `PassThrough` hands it on as a
/// float.
fn options(budget: &Budget, floats: NonFiniteFloatPolicy) -> Options {
    serde_saphyr::options! {
        budget: Some(budget.clone()),
        strict_booleans: true,
        duplicate_keys: DuplicateKeyPolicy::Error,
        merge_keys: MergeKeyPolicy::Error,
        non_finite_float_policy: floats,
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

/// What one input keeps for placing: whether the text passed the limits, and
/// the yamled document built after it did.
///
/// The document is derived from the text alone, so a clone starts with the
/// same contents, `Debug` shows them, and two inputs with the same fields are
/// equal whether or not either has been placed on. The verdict depends on the
/// limits, which a later repository file, process default or [`Input::limits`]
/// can change, so it is kept with the limits it was computed under.
#[derive(Debug, Default)]
pub(super) struct Located {
    /// The limits the last check ran under, and whether the text passed them.
    verdict: Mutex<Option<(Limits, bool)>>,
    /// The document, built the first time the text passes.
    document: OnceLock<Option<yamled::Document>>,
}

impl Clone for Located {
    fn clone(&self) -> Self {
        Self {
            verdict: Mutex::new(self.verdict()),
            document: self.document.clone(),
        }
    }
}

impl PartialEq for Located {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Located {}

impl Located {
    fn verdict(&self) -> Option<(Limits, bool)> {
        self.verdict
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Whether `input` passes `limits`, read again when `limits` differ from
    /// the ones the kept verdict was computed under.
    fn passes(&self, input: &Input, limits: &Limits) -> bool {
        let mut verdict = self.verdict.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((seen, passed)) = verdict.as_ref()
            && seen == limits
        {
            return *passed;
        }
        let passed = within(input, &limits.budget());
        *verdict = Some((limits.clone(), passed));
        passed
    }
}

/// The 1-based line and column, in the text, of each path or of its nearest
/// written ancestor. `None` for every path when the text does not parse or
/// goes past `limits`.
///
/// The document is built by the first call on `input` whose limits the text
/// passes, and read by every call after it. A text `limits` refuse is never
/// handed to yamled, whose parse has no budget of its own. The verdict is kept
/// with the limits it was computed under and read again when they change, so
/// the result follows the limits in force, for the input and for its clones.
pub(super) fn places(
    input: &Input,
    limits: &Limits,
    paths: &[FieldPath],
    at: At,
) -> Vec<Option<(usize, usize)>> {
    let document = document(input, limits);
    paths
        .iter()
        .map(|path| {
            document
                .and_then(|document| match at {
                    At::Value => document.locate_nearest(&yamled_path(path.steps())),
                    At::Key => nearest_key(document, path.steps()),
                })
                .map(|place| (place.line, place.column))
        })
        .collect()
}

/// The yamled document of `input`, when the text passes `limits`.
///
/// It is built by the first call whose limits the text passes, and read by
/// every call after it, so placing and listing the top-level entries share one
/// parse.
pub(super) fn document<'a>(input: &'a Input, limits: &Limits) -> Option<&'a yamled::Document> {
    let located = &input.yaml_document;
    located
        .passes(input, limits)
        .then(|| {
            located
                .document
                .get_or_init(|| yamled::Document::parse(&input.text).ok())
                .as_ref()
        })
        .flatten()
}

/// Whether serde-saphyr reads the whole text under `budget`, depth and node
/// count included, as [`parse`] would, without building any value.
///
/// A non-finite float passes, since it takes no budget and the caller decides
/// what to make of it.
fn within(input: &Input, budget: &Budget) -> bool {
    let options = options(budget, NonFiniteFloatPolicy::PassThrough);
    serde_saphyr::from_str_with_options::<IgnoredAny>(&input.text, options).is_ok()
}

/// Where the last step of `steps` is written as a key, or where the last step
/// that exists is. An item and the root have no key, so they give their value.
fn nearest_key(document: &yamled::Document, mut steps: &[Step]) -> Option<yamled::Location> {
    loop {
        let path = yamled_path(steps);
        if let Some(node) = document.node(&path) {
            return Some(node.key().unwrap_or_else(|| node.value()));
        }
        match steps.split_last() {
            Some((_, rest)) => steps = rest,
            None => return document.locate_nearest(&path),
        }
    }
}

/// `steps` as a yamled path.
fn yamled_path(steps: &[Step]) -> yamled::Path {
    steps
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
        assert!(input.yaml_document.document.get().is_none());
        assert_eq!(input.place(&path).map(|at| at.line), Some(3));
        let first = input.yaml_document.document.get().unwrap();
        assert!(first.is_some());
        assert_eq!(input.place(&path).map(|at| at.line), Some(3));
        let second = input.yaml_document.document.get().unwrap();
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
    fn listing_the_top_level_builds_the_document_that_placing_reuses() {
        let input = Input::new("a.yaml", "a: 1\n");
        assert!(input.yaml_document.document.get().is_none());
        input.top_level().unwrap();
        let first = input.yaml_document.document.get().unwrap();
        assert!(input.place(&FieldPath::root().key("a")).is_some());
        let second = input.yaml_document.document.get().unwrap();
        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn a_text_over_the_size_limit_builds_no_document() {
        let limits = Limits {
            max_reader_input_bytes: Some(4),
            ..Limits::NONE
        };
        let (input, at) = placed_with("a: 1\nb: 2\n", limits);
        assert_eq!(at, None);
        assert_eq!(input.place_key(&FieldPath::root().key("a")), None);
        assert!(input.yaml_document.document.get().is_none());
    }

    #[test]
    fn a_text_over_the_depth_or_node_limit_builds_no_document() {
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
            assert!(input.yaml_document.document.get().is_none());
        }
    }

    fn tight() -> Limits {
        Limits {
            max_depth: Some(2),
            ..Limits::NONE
        }
    }

    #[test]
    fn a_clone_tightened_after_the_document_was_built_is_refused() {
        let text = "a: {b: {c: {d: 1}}}\n";
        let first = FieldPath::root().key("a");
        let loose = Input::new("a.yaml", text);
        assert!(loose.place(&first).is_some());
        let tightened = loose.clone().limits(tight());
        assert!(tightened.place(&first).is_none());
        assert!(loose.place(&first).is_some());
    }

    #[test]
    fn a_refusal_ends_when_the_limits_that_gave_it_do() {
        let text = "a: {b: {c: {d: 1}}}\n";
        let first = FieldPath::root().key("a");
        let refused = Input::new("a.yaml", text).limits(tight());
        assert!(refused.place(&first).is_none());
        let freed = refused.clone().limits(Limits::NONE);
        assert!(freed.place(&first).is_some());
        assert!(refused.place(&first).is_none());
    }

    #[test]
    fn an_input_equals_its_clone_whether_or_not_it_was_placed_on() {
        let input = Input::new("a.yaml", "a: 1\n");
        let fresh = input.clone();
        assert!(input.place(&FieldPath::root().key("a")).is_some());
        assert_eq!(input, fresh);
    }
}

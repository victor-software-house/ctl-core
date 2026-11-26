//! The YAML [`Format`]: serde-saphyr's located reader builds the tree, and
//! serde-saphyr's scalar rules resolve each scalar. No YAML rule is written
//! here.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::OnceLock;

use serde_saphyr::granit_parser::ScalarStyle as Written;
use serde_saphyr::located::{self, MappingNode, ScalarNode, SequenceNode};
use serde_saphyr::scalars::{Inferred, Rules, Scalar as Spelled};
use serde_saphyr::{
    DefaultMessageFormatter, DuplicateKeyPolicy, Error, Localizer, Location, MergeKeyPolicy,
    MessageFormatter, NonFiniteFloatPolicy, Options,
};

use super::exceeded::Exceeded;
use super::format::{Fault, Format, Mismatch, Raise, Reading, Refusal, Resolved, Wanted};
use super::limits::Limits;
use super::messages::printable;
use super::tree::{Node, Scalar, ScalarStyle, Tag};

/// YAML, read by serde-saphyr's located reader. Its extensions are `yaml` and
/// `yml`.
#[derive(Clone, Copy, Debug)]
pub struct Yaml;

/// The YAML format, for [`Formats::add`](crate::Formats::add).
pub static YAML: Yaml = Yaml;

impl Format for Yaml {
    fn name(&self) -> &'static str {
        "yaml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["yaml", "yml"]
    }

    fn read(&self, text: &str, limits: &Limits, reading: Reading) -> Result<Node, Refusal> {
        let bounds = limits.bounds();
        if text.len() > bounds.input_bytes {
            let exceeded = Exceeded::input_bytes(bounds.input_bytes, Raise::FOR_TEXT);
            return Err(Refusal::new(exceeded.fault(), None, None));
        }
        located::read(text, options(limits, reading))
            .map(|node| convert(text, &node))
            .map_err(|error| refusal(text, limits, &error))
    }

    fn resolve<'a>(
        &self,
        scalar: &'a Scalar,
        tag: Option<&Tag>,
        wanted: Wanted,
    ) -> Result<Resolved<'a>, Mismatch> {
        let spelled = Spelled {
            value: scalar.value(),
            style: spelled_style(scalar.style()),
            tag: tag.map(Tag::as_str),
        };
        let rules = rules();
        let at = Location::UNKNOWN;
        let mismatch = |error: Error| {
            let detail = if let Error::NonFiniteFloat { value, .. } = &error {
                printable(&format!("non-finite float `{value}` is not allowed"))
            } else {
                plain(&DefaultMessageFormatter.with_localizer(&Unplaced), &error)
            };
            Mismatch::new(wanted).with_detail(detail)
        };
        match wanted {
            Wanted::Any => match rules.infer(&spelled, at).map_err(mismatch)? {
                Inferred::Null => Ok(Resolved::Null),
                Inferred::Bool(value) => Ok(Resolved::Bool(value)),
                Inferred::Signed(value) => Ok(Resolved::Int(i128::from(value))),
                Inferred::Unsigned(value) => Ok(Resolved::Int(i128::from(value))),
                Inferred::Float(value) => Ok(Resolved::Float(value)),
                Inferred::String(Cow::Owned(text)) => Ok(Resolved::Str(Cow::Owned(text))),
                Inferred::String(Cow::Borrowed(_)) => {
                    Ok(Resolved::Str(Cow::Borrowed(scalar.value())))
                }
            },
            Wanted::Bool => rules
                .boolean(&spelled, at)
                .map(Resolved::Bool)
                .map_err(mismatch),
            Wanted::Int => match rules.signed::<i128>(&spelled, at) {
                Ok(value) => Ok(Resolved::Int(value)),
                Err(_) => rules
                    .unsigned::<u128>(&spelled, at)
                    .map(Resolved::Uint)
                    .map_err(mismatch),
            },
            Wanted::Float => rules
                .float64(&spelled, at)
                .map(Resolved::Float)
                .map_err(mismatch),
            Wanted::Null => rules
                .is_null(&spelled)
                .then_some(Resolved::Null)
                .ok_or_else(|| Mismatch::new(wanted)),
            Wanted::Str => match rules.string(&spelled, at).map_err(mismatch)? {
                Cow::Owned(text) => Ok(Resolved::Str(Cow::Owned(text))),
                Cow::Borrowed(_) => Ok(Resolved::Str(Cow::Borrowed(scalar.value()))),
            },
        }
    }
}

/// The scalar rules under the strict options, built once.
fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| Rules::new(&options(&Limits::NONE, Reading::Strict)))
}

/// The scalar style the YAML rules read, for a style of the tree.
fn spelled_style(style: ScalarStyle) -> Written {
    match style {
        ScalarStyle::Plain => Written::Plain,
        ScalarStyle::SingleQuoted | ScalarStyle::MultilineLiteral => Written::SingleQuoted,
        ScalarStyle::DoubleQuoted | ScalarStyle::MultilineBasic => Written::DoubleQuoted,
        ScalarStyle::Literal => Written::Literal,
        ScalarStyle::Folded => Written::Folded,
    }
}

/// The style of the tree for a YAML style, one to one.
fn style(written: Written) -> ScalarStyle {
    match written {
        Written::Plain => ScalarStyle::Plain,
        Written::SingleQuoted => ScalarStyle::SingleQuoted,
        Written::DoubleQuoted => ScalarStyle::DoubleQuoted,
        Written::Literal => ScalarStyle::Literal,
        Written::Folded => ScalarStyle::Folded,
    }
}

/// The bytes of `text` at `location`: the byte offsets the parser gave, or
/// else an empty range at the offset its line and column name, which is all
/// the parser gives a scanner error. `None` for a location it never set.
fn bytes(text: &str, location: Location) -> Option<Range<usize>> {
    let span = location.span();
    if let (Some(start), Some(length)) = (span.byte_offset(), span.byte_len()) {
        let start = usize::try_from(start).ok()?;
        let length = usize::try_from(length).ok()?;
        return Some(start..start.saturating_add(length));
    }
    let offset = offset_of(text, location)?;
    Some(offset..offset)
}

/// The byte offset in `text` of the 1-based line and character column of
/// `location`, as the parser counts them: a byte order mark that starts line 1
/// is no column. A column past the end of its line is the end of the line.
fn offset_of(text: &str, location: Location) -> Option<usize> {
    let line = usize::try_from(location.line()).ok()?.checked_sub(1)?;
    let column = usize::try_from(location.column()).ok()?.checked_sub(1)?;
    let start: usize = text.split_inclusive('\n').take(line).map(str::len).sum();
    let content = text.get(start..)?.split('\n').next()?;
    let (bom, counted) = match content.strip_prefix('\u{feff}') {
        Some(rest) if start == 0 => (content.len() - rest.len(), rest),
        _ => (0, content),
    };
    let within = counted
        .char_indices()
        .nth(column)
        .map_or(counted.len(), |(index, _)| index);
    Some(start + bom + within)
}

/// The bytes a node is written at. The parser locates every node it builds,
/// so the empty range at the start is for a node it never located.
fn placed(text: &str, location: Location) -> Range<usize> {
    bytes(text, location).unwrap_or_default()
}

/// The tree for a located node, with each replayed node marked by the alias
/// that brought it.
fn convert(text: &str, node: &located::Node) -> Node {
    let locations = node.locations();
    let span = placed(text, locations.defined_location);
    let built = match node {
        located::Node::Scalar(ScalarNode {
            value,
            style: written,
            ..
        }) => Node::scalar(span, Scalar::new(value.as_str(), style(*written))),
        located::Node::Sequence(SequenceNode { items, .. }) => {
            Node::sequence(span, items.iter().map(|item| convert(text, item)).collect())
        }
        located::Node::Mapping(MappingNode { entries, .. }) => Node::mapping(
            span,
            entries
                .iter()
                .map(|(key, value)| (convert(text, key), convert(text, value)))
                .collect(),
        ),
    };
    let built = match node.tag() {
        Some(tag) => built.with_tag(Tag::new(tag)),
        None => built,
    };
    if locations.reference_location == locations.defined_location {
        built
    } else {
        built.with_used_at(placed(text, locations.reference_location))
    }
}

/// The refusal for `error`, by its variant. An alias error sits at the alias
/// and names where its value is defined, from the error the aliased value
/// raised, found under any alias reached through another alias.
fn refusal(text: &str, limits: &Limits, error: &Error) -> Refusal {
    let mut error = error;
    while let Error::WithSnippet { error: inner, .. } = error {
        error = inner;
    }
    let Error::AliasError {
        error: inner,
        locations,
        ..
    } = error
    else {
        let span = error
            .location()
            .filter(|at| *at != Location::UNKNOWN)
            .and_then(|at| bytes(text, at));
        return Refusal::new(fault(text, limits, error), span, None);
    };
    let mut failing = inner.as_ref();
    while let Error::AliasError { error, .. } | Error::WithSnippet { error, .. } = failing {
        failing = error;
    }
    let defined = failing
        .location()
        .filter(|at| *at != Location::UNKNOWN)
        .unwrap_or(locations.defined_location);
    Refusal::new(
        fault(text, limits, failing),
        bytes(text, locations.reference_location),
        bytes(text, defined),
    )
}

/// The fault for `error`, by its variant and never by its text. A key that has
/// no text of its own reads as it is written.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "Error is non_exhaustive: any other error is a syntax fault with serde-saphyr's wording"
)]
fn fault(text: &str, limits: &Limits, error: &Error) -> Fault {
    match error {
        Error::DuplicateMappingKey { key, location } => Fault::DuplicateKey {
            key: key.clone().unwrap_or_else(|| {
                bytes(text, *location)
                    .and_then(|range| text.get(range))
                    .unwrap_or_default()
                    .to_owned()
            }),
        },
        Error::MergeKeyNotAllowed { .. } => Fault::MergeKey,
        Error::MultipleDocuments { .. } => Fault::Syntax {
            detail: "the text holds more than one YAML document".to_owned(),
        },
        _ => Exceeded::of(
            error,
            &limits.budget(),
            &limits.alias_limits(),
            Raise::FOR_TEXT,
        )
        .map_or_else(
            || Fault::Syntax {
                detail: plain(&DefaultMessageFormatter.with_localizer(&Unplaced), error),
            },
            |exceeded| exceeded.fault(),
        ),
    }
}

/// What every read of a YAML text uses, under `limits`: its budget and its
/// alias replay limits. A [`Reading::Listing`] also reads a non-finite float
/// as a value.
fn options(limits: &Limits, reading: Reading) -> Options {
    let (duplicates, merges, floats) = match reading {
        Reading::Strict => (
            DuplicateKeyPolicy::Error,
            MergeKeyPolicy::Error,
            NonFiniteFloatPolicy::Default,
        ),
        Reading::Listing => (
            DuplicateKeyPolicy::PassThrough,
            MergeKeyPolicy::AsOrdinary,
            NonFiniteFloatPolicy::PassThrough,
        ),
    };
    serde_saphyr::options! {
        budget: Some(limits.budget()),
        alias_limits: limits.alias_limits(),
        strict_booleans: true,
        duplicate_keys: duplicates,
        merge_keys: merges,
        non_finite_float_policy: floats,
        with_snippet: false,
    }
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
    printable(&formatter.format_message(error))
}

/// serde-saphyr's wording without the position it appends. That position
/// counts from the parsed text, which for frontmatter is not the file, and a
/// [`Refusal`] already carries the bytes it sits at.
struct Unplaced;

impl Localizer for Unplaced {
    fn attach_location<'a>(&self, base: Cow<'a, str>, _: Location) -> Cow<'a, str> {
        base
    }

    fn alias_used_at(&self, _: Location) -> String {
        String::new()
    }
}

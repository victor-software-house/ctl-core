//! The YAML [`Format`]: serde-saphyr's located reader builds the tree, and
//! serde-saphyr's scalar rules resolve each scalar. No YAML rule is written
//! here.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::OnceLock;

use serde_saphyr::granit_parser::ScalarStyle as Written;
use serde_saphyr::located::{self, MappingNode, ScalarNode, SequenceNode};
use serde_saphyr::scalars::{Inferred, Rules, Scalar as Spelled};
use serde_saphyr::{DefaultMessageFormatter, Error, Location};

use super::exceeded::{Exceeded, Raise};
use super::format::{Fault, Format, Mismatch, Reading, Refusal, Resolved, Wanted};
use super::limits::Limits;
use super::tree::{Node, Scalar, ScalarStyle, Tag};
use super::yaml_text::{self, Unplaced};

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
        located::read(text, yaml_text::options(limits, reading))
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
        let mismatch = Mismatch::new(wanted);
        match wanted {
            Wanted::Any => match rules.infer(&spelled, at).map_err(|_error| mismatch)? {
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
                .map_err(|_error| mismatch),
            Wanted::Int => match rules.signed::<i128>(&spelled, at) {
                Ok(value) => Ok(Resolved::Int(value)),
                Err(_) => rules
                    .unsigned::<u128>(&spelled, at)
                    .map(Resolved::Uint)
                    .map_err(|_error| mismatch),
            },
            Wanted::Float => rules
                .float64(&spelled, at)
                .map(Resolved::Float)
                .map_err(|_error| mismatch),
            Wanted::Null => rules
                .is_null(&spelled)
                .then_some(Resolved::Null)
                .ok_or(mismatch),
            Wanted::Str => {
                if rules.is_null(&spelled) {
                    Err(mismatch)
                } else {
                    Ok(Resolved::Str(Cow::Borrowed(scalar.value())))
                }
            }
        }
    }
}

/// The scalar rules under the strict options, built once.
fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| Rules::new(&yaml_text::options(&Limits::NONE, Reading::Strict)))
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
/// `location`, as the parser counts them. A column past the end of its line
/// is the end of the line.
fn offset_of(text: &str, location: Location) -> Option<usize> {
    let line = usize::try_from(location.line()).ok()?.checked_sub(1)?;
    let column = usize::try_from(location.column()).ok()?.checked_sub(1)?;
    let start: usize = text.split_inclusive('\n').take(line).map(str::len).sum();
    let content = text.get(start..)?.split('\n').next()?;
    let within = content
        .char_indices()
        .nth(column)
        .map_or(content.len(), |(index, _)| index);
    Some(start + within)
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
        _ => Exceeded::of(
            error,
            &limits.budget(),
            &limits.alias_limits(),
            Raise::FOR_TEXT,
        )
        .map_or_else(
            || Fault::Syntax {
                detail: yaml_text::plain(&DefaultMessageFormatter.with_localizer(&Unplaced), error),
            },
            |exceeded| exceeded.fault(),
        ),
    }
}

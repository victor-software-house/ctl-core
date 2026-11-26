//! The TOML [`Format`]: the `toml` crate's span-carrying tree becomes the
//! located tree, and each scalar resolves by how it is written.

use std::ops::Range;

use toml::Spanned;
use toml::de::{DeString, DeTable, DeValue};

use super::exceeded::{Exceeded, Raise};
use super::format::{Fault, Format, Mismatch, Reading, Refusal, Resolved, Wanted};
use super::limits::Limits;
use super::toml_text::Measure;
use super::tree::{Node, Scalar, ScalarStyle, Tag};

/// TOML, read by the `toml` crate. Its extension is `toml`.
#[derive(Clone, Copy, Debug)]
pub struct Toml;

/// The TOML format, for [`Formats::add`](crate::Formats::add) and
/// [`Input::new_with_format`](crate::Input::new_with_format).
pub static TOML: Toml = Toml;

impl Format for Toml {
    fn name(&self) -> &'static str {
        "toml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["toml"]
    }

    /// TOML has no duplicate key and no merge key, so both readings give the
    /// same tree.
    fn read(&self, text: &str, limits: &Limits, _reading: Reading) -> Result<Node, Refusal> {
        let bounds = limits.bounds();
        if text.len() > bounds.input_bytes {
            let exceeded = Exceeded::input_bytes(bounds.input_bytes, Raise::FOR_TEXT);
            return Err(Refusal::new(exceeded.fault(), None, None));
        }
        let root = DeTable::parse(text).map_err(|error| {
            let detail = error.message().to_owned();
            Refusal::new(Fault::Syntax { detail }, error.span(), None)
        })?;
        Measure::new(bounds, Raise::FOR_TEXT)
            .table(&root.span(), root.get_ref(), 1)
            .map_err(|refused| Refusal::new(refused.exceeded.fault(), Some(refused.at), None))?;
        Ok(table(text, root.span(), root.get_ref()))
    }

    fn resolve<'a>(
        &self,
        scalar: &'a Scalar,
        _tag: Option<&Tag>,
        wanted: Wanted,
    ) -> Result<Resolved<'a>, Mismatch> {
        let mismatch = Mismatch::new(wanted);
        let text = scalar.value();
        let spelling = match scalar.style() {
            ScalarStyle::Plain => Plain::of(text),
            ScalarStyle::SingleQuoted
            | ScalarStyle::DoubleQuoted
            | ScalarStyle::Literal
            | ScalarStyle::Folded
            | ScalarStyle::MultilineBasic
            | ScalarStyle::MultilineLiteral => Plain::Quoted,
        };
        let resolved = match (wanted, spelling) {
            (Wanted::Any | Wanted::Bool, Plain::Bool(value)) => Some(Resolved::Bool(value)),
            (Wanted::Any | Wanted::Int, Plain::Int(value)) => Some(Resolved::Int(value)),
            (Wanted::Any | Wanted::Float, Plain::Float(value)) => Some(Resolved::Float(value)),
            (Wanted::Float, Plain::Int(_)) => float(text).map(Resolved::Float),
            (Wanted::Any | Wanted::Str, Plain::Quoted | Plain::Datetime) => {
                Some(Resolved::Str(text))
            }
            _ => None,
        };
        resolved.ok_or(mismatch)
    }
}

/// How a scalar reads from its syntax alone: a quoted style is a string, and a
/// plain one is a boolean, an integer, a float, or else a datetime.
#[derive(Clone, Copy)]
enum Plain {
    Quoted,
    Bool(bool),
    Int(i128),
    Float(f64),
    Datetime,
}

impl Plain {
    /// The reading of a plain scalar's canonical text, which `read` wrote from
    /// the `toml` crate's own parse, so no TOML number syntax is read here.
    fn of(text: &str) -> Self {
        match text {
            "true" => Self::Bool(true),
            "false" => Self::Bool(false),
            _ => text.parse::<i128>().map_or_else(
                |_| float(text).map_or(Self::Datetime, Self::Float),
                Self::Int,
            ),
        }
    }
}

/// The float a canonical integer or float text spells. An overflow to infinity
/// is refused unless the text spells infinity, as the `toml` crate refuses it.
fn float(text: &str) -> Option<f64> {
    text.parse::<f64>()
        .ok()
        .filter(|value| !value.is_infinite() || text.contains("inf"))
}

/// The mapping for `table`, its entries in the order written: a parsed table
/// iterates by key, not by position.
fn table(text: &str, span: Range<usize>, table: &DeTable<'_>) -> Node {
    let mut entries: Vec<_> = table.iter().collect();
    entries.sort_by_key(|(key, _)| key.span().start);
    Node::mapping(
        span,
        entries
            .into_iter()
            .map(|(key, value)| (key_node(text, key), value_node(text, value)))
            .collect(),
    )
}

/// A key as a scalar of its processed text.
fn key_node(text: &str, key: &Spanned<DeString<'_>>) -> Node {
    let span = key.span();
    let style = style(text, &span);
    Node::scalar(span, Scalar::new(key.get_ref().to_string(), style))
}

/// The node for a value. A number is its canonical decimal text, so a hex
/// `0xff` is `255`, and one the `i128` cannot hold keeps the digits as
/// written, which no integer field reads. A datetime is its text.
fn value_node(text: &str, value: &Spanned<DeValue<'_>>) -> Node {
    let span = value.span();
    let plain =
        |written: String| Node::scalar(span.clone(), Scalar::new(written, ScalarStyle::Plain));
    match value.get_ref() {
        DeValue::Table(entries) => table(text, span.clone(), entries),
        DeValue::Array(items) => Node::sequence(
            span.clone(),
            items.iter().map(|item| value_node(text, item)).collect(),
        ),
        DeValue::String(string) => Node::scalar(
            span.clone(),
            Scalar::new(string.to_string(), style(text, &span)),
        ),
        DeValue::Integer(integer) => plain(
            i128::from_str_radix(integer.as_str(), integer.radix())
                .map_or_else(|_| integer.as_str().to_owned(), |number| number.to_string()),
        ),
        DeValue::Float(number) => plain(number.as_str().to_owned()),
        DeValue::Boolean(boolean) => plain(boolean.to_string()),
        DeValue::Datetime(datetime) => plain(datetime.to_string()),
    }
}

/// How the string or key written at `span` is quoted, from its first bytes.
fn style(text: &str, span: &Range<usize>) -> ScalarStyle {
    let written = text.get(span.clone()).unwrap_or_default();
    if written.starts_with("\"\"\"") {
        ScalarStyle::MultilineBasic
    } else if written.starts_with("'''") {
        ScalarStyle::MultilineLiteral
    } else if written.starts_with('"') {
        ScalarStyle::DoubleQuoted
    } else if written.starts_with('\'') {
        ScalarStyle::SingleQuoted
    } else {
        ScalarStyle::Plain
    }
}

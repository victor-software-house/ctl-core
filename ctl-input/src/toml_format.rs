//! The TOML [`Format`]: the `toml` crate's span-carrying tree becomes the
//! located tree, each scalar carrying the type its syntax gave it, and each
//! scalar resolves by that type.

use std::borrow::Cow;
use std::ops::Range;

use toml::Spanned;
use toml::de::{DeInteger, DeString, DeTable, DeValue};

use super::exceeded::Exceeded;
use super::format::{Fault, Format, Mismatch, Raise, Reading, Refusal, Resolved, Wanted};
use super::limits::{Bounds, Limits};
use super::tree::{Node, Scalar, ScalarStyle, ScalarSyntax, Tag};

/// TOML, read by the `toml` crate. Its extension is `toml`.
#[derive(Clone, Copy, Debug)]
pub struct Toml;

/// The TOML format, for [`Formats::add`](crate::Formats::add).
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

    /// Resolves by the type the scalar's syntax gave it, and never by its
    /// text. A scalar that carries no syntax, such as one another format built,
    /// is text.
    fn resolve<'a>(
        &self,
        scalar: &'a Scalar,
        _tag: Option<&Tag>,
        wanted: Wanted,
    ) -> Result<Resolved<'a>, Mismatch> {
        let text = scalar.value();
        let resolved = match (scalar.syntax().unwrap_or(ScalarSyntax::Text), wanted) {
            (ScalarSyntax::Text | ScalarSyntax::Datetime, Wanted::Any | Wanted::Str) => {
                Some(Resolved::Str(Cow::Borrowed(text)))
            }
            (ScalarSyntax::Boolean, Wanted::Any | Wanted::Bool) => {
                Some(Resolved::Bool(text == "true"))
            }
            (ScalarSyntax::Integer, Wanted::Any | Wanted::Int) => integer(text),
            (ScalarSyntax::Integer, Wanted::Float) => {
                integer(text).and_then(|_| text.parse().ok().map(Resolved::Float))
            }
            (ScalarSyntax::Float, Wanted::Any | Wanted::Float) => float(text),
            _ => None,
        };
        resolved.ok_or(Mismatch::new(wanted))
    }
}

/// The integer a canonical integer text spells: an `i128`, or a `u128` above
/// it. A text no `u128` holds is no integer.
fn integer(text: &str) -> Option<Resolved<'static>> {
    text.parse()
        .map(Resolved::Int)
        .or_else(|_| text.parse().map(Resolved::Uint))
        .ok()
}

/// The float a float's text spells. An overflow to infinity is refused unless
/// the text spells infinity, as the `toml` crate refuses it.
fn float(text: &str) -> Option<Resolved<'static>> {
    text.parse::<f64>()
        .ok()
        .filter(|value| !value.is_infinite() || text.contains("inf"))
        .map(Resolved::Float)
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
    let scalar = Scalar::new(key.get_ref().to_string(), style).with_syntax(ScalarSyntax::Text);
    Node::scalar(span, scalar)
}

/// The node for a value, with the type its syntax gave it. An integer is its
/// canonical decimal text, so a hex `0xff` is `255`; one no `u128` holds keeps
/// its written digits and radix prefix, which no integer reads. A datetime is
/// its text.
fn value_node(text: &str, value: &Spanned<DeValue<'_>>) -> Node {
    let span = value.span();
    let plain = |written: String, syntax: ScalarSyntax| {
        let scalar = Scalar::new(written, ScalarStyle::Plain).with_syntax(syntax);
        Node::scalar(span.clone(), scalar)
    };
    match value.get_ref() {
        DeValue::Table(entries) => table(text, span.clone(), entries),
        DeValue::Array(items) => Node::sequence(
            span.clone(),
            items.iter().map(|item| value_node(text, item)).collect(),
        ),
        DeValue::String(string) => {
            let scalar =
                Scalar::new(string.to_string(), style(text, &span)).with_syntax(ScalarSyntax::Text);
            Node::scalar(span.clone(), scalar)
        }
        DeValue::Integer(integer) => plain(canonical(integer), ScalarSyntax::Integer),
        DeValue::Float(number) => plain(number.as_str().to_owned(), ScalarSyntax::Float),
        DeValue::Boolean(boolean) => plain(boolean.to_string(), ScalarSyntax::Boolean),
        DeValue::Datetime(datetime) => plain(datetime.to_string(), ScalarSyntax::Datetime),
    }
}

/// The decimal text of an integer, from an `i128` or else a `u128`, or its
/// written digits with their radix prefix when no `u128` holds it.
fn canonical(integer: &DeInteger<'_>) -> String {
    let (digits, radix) = (integer.as_str(), integer.radix());
    i128::from_str_radix(digits, radix)
        .map(|number| number.to_string())
        .or_else(|_| u128::from_str_radix(digits, radix).map(|number| number.to_string()))
        .unwrap_or_else(|_| integer.to_string())
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

/// A limit a tree went past, and where in the text.
struct Refused {
    exceeded: Exceeded,
    at: Range<usize>,
}

/// Walks a tree in the order it was written, counting depth and nodes
/// against the limits in force. `max_depth` is YAML's rule too: a table or
/// an array is one level, and the root table is the first.
struct Measure {
    max_depth: usize,
    max_nodes: usize,
    nodes: usize,
    raise: Raise,
}

impl Measure {
    /// A count at zero nodes under `bounds`.
    fn new(bounds: Bounds, raise: Raise) -> Self {
        Self {
            max_depth: bounds.depth,
            max_nodes: bounds.nodes,
            nodes: 0,
            raise,
        }
    }

    /// A table at `depth`, then its entries in the order they were written:
    /// a parsed table iterates by key, not by position.
    fn table(
        &mut self,
        span: &Range<usize>,
        table: &DeTable<'_>,
        depth: usize,
    ) -> Result<(), Refused> {
        self.container(span, depth)?;
        let mut entries: Vec<_> = table.iter().collect();
        entries.sort_by_key(|(key, _)| key.span().start);
        entries
            .into_iter()
            .try_for_each(|(_, value)| self.value(value, depth + 1))
    }

    fn value(&mut self, value: &Spanned<DeValue<'_>>, depth: usize) -> Result<(), Refused> {
        let span = value.span();
        match value.get_ref() {
            DeValue::Table(table) => self.table(&span, table, depth),
            DeValue::Array(items) => {
                self.container(&span, depth)?;
                items
                    .iter()
                    .try_for_each(|item| self.value(item, depth + 1))
            }
            DeValue::String(_)
            | DeValue::Integer(_)
            | DeValue::Float(_)
            | DeValue::Boolean(_)
            | DeValue::Datetime(_) => self.node(&span),
        }
    }

    /// A table or an array: one level deeper, and one node.
    fn container(&mut self, span: &Range<usize>, depth: usize) -> Result<(), Refused> {
        if depth > self.max_depth {
            return Err(self.refuse("max_depth", self.max_depth, span));
        }
        self.node(span)
    }

    fn node(&mut self, span: &Range<usize>) -> Result<(), Refused> {
        self.nodes += 1;
        if self.nodes > self.max_nodes {
            return Err(self.refuse("max_nodes", self.max_nodes, span));
        }
        Ok(())
    }

    fn refuse(&self, field: &'static str, limit: usize, span: &Range<usize>) -> Refused {
        Refused {
            exceeded: Exceeded::new(field, limit, self.raise),
            at: span.clone(),
        }
    }
}

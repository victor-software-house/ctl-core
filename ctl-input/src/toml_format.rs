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
use super::tree::{Node, NodeKind, Scalar, ScalarStyle, ScalarSyntax, Tag};

/// TOML, read by the `toml` crate. Its extension is `toml`.
#[derive(Clone, Copy, Debug)]
pub struct Toml;

/// The TOML format, for [`Formats::add`](crate::Formats::add).
pub static TOML: Toml = Toml;

/// What the `toml` crate's parser says of a key written twice, at the repeated
/// key. The error carries no kind to match instead.
const REPEATED_KEY: &str = "duplicate key";

impl Format for Toml {
    fn name(&self) -> &'static str {
        "toml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["toml"]
    }

    /// TOML forbids a duplicate key and has no merge key, so both readings give
    /// the same tree or refuse the same text.
    fn read(&self, text: &str, limits: &Limits, _reading: Reading) -> Result<Node, Refusal> {
        let bounds = limits.bounds();
        if text.len() > bounds.input_bytes {
            let exceeded = Exceeded::input_bytes(bounds.input_bytes, Raise::FOR_TEXT);
            return Err(Refusal::new(exceeded.fault(), None, None));
        }
        let root = DeTable::parse(text).map_err(|error| {
            let span = error.span();
            let fault = if error.message() == REPEATED_KEY {
                let written = span.clone().and_then(|at| text.get(at)).unwrap_or_default();
                Fault::DuplicateKey {
                    key: key_name(written),
                }
            } else {
                Fault::Syntax {
                    detail: error.message().to_owned(),
                }
            };
            Refusal::new(fault, span, None)
        })?;
        let tree = table(text, root.span(), root.get_ref());
        Measure::new(bounds, Raise::FOR_TEXT)
            .node(&tree, 1)
            .map_err(|refused| Refusal::new(refused.exceeded.fault(), Some(refused.at), None))?;
        Ok(tree)
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

/// The name of the key written as `written`, as TOML reads it, without quotes
/// or escapes: the last key of the dotted path the `toml` crate builds from
/// `written = 0`. Text that reads as no key is returned as written.
fn key_name(written: &str) -> String {
    let entry = format!("{written} = 0");
    let Ok(root) = DeTable::parse(&entry) else {
        return written.to_owned();
    };
    let mut name = None;
    let mut level = root.get_ref();
    while let Some((key, value)) = level.iter().next() {
        name = Some(key.get_ref().to_string());
        let DeValue::Table(inner) = value.get_ref() else {
            break;
        };
        level = inner;
    }
    name.unwrap_or_else(|| written.to_owned())
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
    entries.sort_unstable_by_key(|(key, _)| key.span().start);
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

/// Walks the tree in the order it was written, counting depth and nodes
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

    /// A node at `depth`, then its children in the order they were written.
    fn node(&mut self, node: &Node, depth: usize) -> Result<(), Refused> {
        let span = node.span();
        match node.kind() {
            NodeKind::Mapping(entries) => {
                self.container(&span, depth)?;
                entries
                    .iter()
                    .try_for_each(|(_, value)| self.node(value, depth + 1))
            }
            NodeKind::Sequence(items) => {
                self.container(&span, depth)?;
                items.iter().try_for_each(|item| self.node(item, depth + 1))
            }
            NodeKind::Scalar(_) => self.count(&span),
        }
    }

    /// A table or an array: one level deeper, and one node.
    fn container(&mut self, span: &Range<usize>, depth: usize) -> Result<(), Refused> {
        if depth > self.max_depth {
            return Err(self.refuse("max_depth", self.max_depth, span));
        }
        self.count(span)
    }

    fn count(&mut self, span: &Range<usize>) -> Result<(), Refused> {
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

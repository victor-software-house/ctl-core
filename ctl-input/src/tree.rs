//! The located tree every [format](crate::format::Format) reads a text into.
//!
//! A node keeps the bytes of the text it was written in, so a problem found
//! later, in any format, is placed from the tree alone. A scalar keeps its
//! processed value and the style it was written in, and a mapping keeps its
//! key nodes beside its value nodes, in the order written.
//!
//! A format that expands aliases while it reads builds no alias node: a node
//! it replays keeps the span of the node it copies, and
//! [`Node::used_at`] holds where the alias was written.
//!
//! Every type is read-only and non-exhaustive, so a format outside this crate
//! builds nodes through [`Node::scalar`], [`Node::sequence`],
//! [`Node::mapping`], [`Scalar::new`] and [`Tag::new`], and a later release
//! can add a field.

use std::ops::Range;

use super::{At, Step};

/// One node of a text: its bytes, the alias that replayed it, its tag and what
/// it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Node {
    span: Range<usize>,
    used_at: Option<Range<usize>>,
    tag: Option<Tag>,
    kind: NodeKind,
}

/// What a [`Node`] holds.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeKind {
    /// A scalar, with its processed value and style.
    Scalar(Scalar),
    /// The items of a sequence or array, in the order written.
    Sequence(Vec<Node>),
    /// The entries of a mapping or table, as a key node and a value node, in
    /// the order written.
    Mapping(Vec<(Node, Node)>),
}

/// A scalar's processed value and how it was written.
///
/// The value has its escapes, folding and chomping applied, as a `String`
/// field receives it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scalar {
    value: String,
    style: ScalarStyle,
}

/// How a scalar is written.
///
/// A TOML basic string is [`Self::DoubleQuoted`], a literal string
/// [`Self::SingleQuoted`], and a TOML number, boolean or datetime
/// [`Self::Plain`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScalarStyle {
    /// No quotes: a YAML plain scalar, a bare TOML key, or a TOML number,
    /// boolean or datetime.
    Plain,
    /// A YAML single-quoted scalar, or a TOML literal string `'…'`.
    SingleQuoted,
    /// A YAML double-quoted scalar, or a TOML basic string `"…"`.
    DoubleQuoted,
    /// A YAML literal block scalar, `|`.
    Literal,
    /// A YAML folded block scalar, `>`.
    Folded,
    /// A TOML multi-line basic string, `"""…"""`.
    MultilineBasic,
    /// A TOML multi-line literal string, `'''…'''`.
    MultilineLiteral,
}

/// The tag written on a node, in the spelling of serde-saphyr's `Tagged`:
/// `!override` for a local tag, and the resolved handle and suffix otherwise,
/// such as `tag:yaml.org,2002:str` for `!!str`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag(String);

impl Node {
    /// A scalar node written at `span`.
    #[must_use]
    pub fn scalar(span: Range<usize>, scalar: Scalar) -> Self {
        Self::of(span, NodeKind::Scalar(scalar))
    }

    /// A sequence node written at `span`, with its items in the order
    /// written.
    #[must_use]
    pub fn sequence(span: Range<usize>, items: Vec<Self>) -> Self {
        Self::of(span, NodeKind::Sequence(items))
    }

    /// A mapping node written at `span`, with its key and value nodes in the
    /// order written.
    #[must_use]
    pub fn mapping(span: Range<usize>, entries: Vec<(Self, Self)>) -> Self {
        Self::of(span, NodeKind::Mapping(entries))
    }

    fn of(span: Range<usize>, kind: NodeKind) -> Self {
        Self {
            span,
            used_at: None,
            tag: None,
            kind,
        }
    }

    /// This node with the tag written on it.
    #[must_use]
    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tag = Some(tag);
        self
    }

    /// This node as replayed by the alias written at `used_at`.
    #[must_use]
    pub fn with_used_at(mut self, used_at: Range<usize>) -> Self {
        self.used_at = Some(used_at);
        self
    }

    /// The bytes of the node in the text, quotes and block indicators
    /// included, so `text[node.span()]` is its source. A node an alias
    /// replayed has the span of the node it copies.
    #[must_use]
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// The span of the outermost alias that replayed this node, the one
    /// written where the value is used. Every node beneath a replayed node
    /// inherits it, and it is `None` for a node no alias replayed.
    #[must_use]
    pub fn used_at(&self) -> Option<Range<usize>> {
        self.used_at.clone()
    }

    /// The tag written on the node, if any.
    #[must_use]
    pub fn tag(&self) -> Option<&Tag> {
        self.tag.as_ref()
    }

    /// What the node holds.
    #[must_use]
    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    /// The span of the value `steps` reach, or of the last step that exists.
    /// With [`At::Key`], the span of the key node that names it; an index
    /// step and the root have no key, so they give their value.
    ///
    /// A key step matches the first entry, in the order written, whose key
    /// node is a scalar with exactly that value.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the next change places every problem through the tree"
        )
    )]
    pub(crate) fn nearest(&self, steps: &[Step], at: At) -> Range<usize> {
        let mut found = self;
        let mut key = None;
        for step in steps {
            let next = match (&found.kind, step) {
                (NodeKind::Mapping(entries), Step::Key(name)) => entries
                    .iter()
                    .find(|(entry, _)| entry.names(name))
                    .map(|(entry, value)| (value, Some(entry))),
                (NodeKind::Sequence(items), Step::Index(index)) => {
                    items.get(*index).map(|value| (value, None))
                }
                (NodeKind::Scalar(_) | NodeKind::Mapping(_), _)
                | (NodeKind::Sequence(_), Step::Key(_)) => None,
            };
            let Some((next, named)) = next else { break };
            (found, key) = (next, named);
        }
        match at {
            At::Value => found.span(),
            At::Key => key.unwrap_or(found).span(),
        }
    }

    /// Whether this node is a scalar whose value is exactly `name`.
    fn names(&self, name: &str) -> bool {
        matches!(&self.kind, NodeKind::Scalar(scalar) if scalar.value == name)
    }
}

impl Scalar {
    /// A scalar with its processed `value`, written in `style`.
    #[must_use]
    pub fn new(value: impl Into<String>, style: ScalarStyle) -> Self {
        Self {
            value: value.into(),
            style,
        }
    }

    /// The processed value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// How the scalar was written.
    #[must_use]
    pub fn style(&self) -> ScalarStyle {
        self.style
    }
}

impl Tag {
    /// A tag in the spelling described on [`Tag`].
    #[must_use]
    pub fn new(spelling: impl Into<String>) -> Self {
        Self(spelling.into())
    }

    /// The spelling the tag was built with.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The 1-based line and column, counted in characters, of byte `offset` in
/// `text`. An offset past the end, as at an unclosed array, is the end.
#[cfg_attr(
    not(any(test, feature = "toml")),
    expect(
        dead_code,
        reason = "the next change places every problem through the tree"
    )
)]
pub(crate) fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = String::from_utf8_lossy(&text.as_bytes()[..offset.min(text.len())]);
    let line = before.split('\n').count();
    let column = 1 + before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count());
    (line, column)
}

#[cfg(test)]
mod differential;

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(span: Range<usize>, value: &str) -> Node {
        Node::scalar(span, Scalar::new(value, ScalarStyle::Plain))
    }

    /// `a: [x, y]` as a format would build it.
    fn tree() -> Node {
        let list = Node::sequence(3..9, vec![plain(4..5, "x"), plain(7..8, "y")]);
        Node::mapping(0..9, vec![(plain(0..1, "a"), list)])
    }

    #[test]
    fn a_path_gives_its_value_or_its_key() {
        let steps = [Step::Key("a".to_owned()), Step::Index(1)];
        assert_eq!(tree().nearest(&steps, At::Value), 7..8);
        assert_eq!(tree().nearest(&steps, At::Key), 7..8);
        assert_eq!(tree().nearest(&steps[..1], At::Key), 0..1);
        assert_eq!(tree().nearest(&[], At::Key), 0..9);
    }

    #[test]
    fn a_missing_step_gives_the_last_one_that_exists() {
        let steps = [Step::Key("a".to_owned()), Step::Key("b".to_owned())];
        assert_eq!(tree().nearest(&steps, At::Value), 3..9);
        assert_eq!(tree().nearest(&steps, At::Key), 0..1);
    }

    #[test]
    fn a_position_counts_characters() {
        assert_eq!(position("é: 1\nb", 6), (2, 1));
        assert_eq!(position("é: 1\nb", 3), (1, 3));
    }
}

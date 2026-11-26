//! The top-level entries of a text, each with the byte range of its value,
//! how the value is written and, for a string, its text.
//!
//! A caller that reads a mapping it did not write, such as the front matter of
//! a Markdown file, learns where each value sits and what it reads as, with no
//! scanner of its own. The tree gives the spans and the style, and the
//! text's format decides what a scalar reads as.

use std::ops::Range;

use super::format::{Format, Reading, Resolved, Wanted};
use super::tree::{Node, NodeKind, ScalarStyle, ScalarSyntax};
use super::{Input, InputError, Raise, limits};

/// How the value of an [`Entry`] is written.
///
/// Non-exhaustive: a later release can name another way to write a value.
/// TOML writes a string four ways, which map onto the first two quoted forms
/// and the two multi-line ones, and writes a number, a boolean or a date as
/// [`Written::Plain`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Written {
    /// A YAML plain scalar, or a TOML number, boolean or date.
    Plain,
    /// A single-quoted YAML scalar, or a TOML literal string `'…'`.
    SingleQuoted,
    /// A double-quoted YAML scalar, or a TOML basic string `"…"`.
    DoubleQuoted,
    /// A YAML literal block scalar, `|`.
    Literal,
    /// A YAML folded block scalar, `>`.
    Folded,
    /// A TOML multi-line basic string, `"""…"""`.
    MultilineBasic,
    /// A TOML multi-line literal string, `'''…'''`.
    MultilineLiteral,
    /// A YAML alias, `*name`.
    Alias,
    /// A YAML sequence or mapping, or a TOML array or table.
    Collection,
}

/// One entry of the top-level mapping or table of a text.
///
/// Non-exhaustive: a later release can add a field.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entry {
    /// The key, as the parser read it: quotes and escapes resolved. A YAML key
    /// that is a number, a boolean or a null is its text as written.
    pub key: String,
    /// The bytes of the value in [`Input::text`], as written. A quoted string
    /// includes its quotes and a block scalar its header. An anchor or a tag
    /// before a YAML value is outside it, and an alias is `*name`.
    pub value: Range<usize>,
    /// How the value is written.
    pub written: Written,
    /// The text of the value, when the parser reads it as a string, and `None`
    /// for a number, a boolean, a null, a date, a collection or an alias to
    /// one. A YAML alias to a string, and a scalar under `!!str`, hold the
    /// string.
    pub string: Option<String>,
}

impl Input {
    /// The entries of the text's top-level mapping or table, in the order
    /// written.
    ///
    /// An entry whose key is a YAML sequence, mapping or alias is left out, and
    /// the entries around it keep their own ranges. A text whose root is no
    /// mapping, such as a sequence or a scalar, has no entries.
    ///
    /// The listing refuses nothing that YAML allows, so a key written twice,
    /// a merge key (`<<`) and a `.inf` or `.NaN` are entries like the rest,
    /// where [`Input::parse`] refuses them. The text must pass the limits in
    /// force, as [`Input::parse`] holds it to, and the input reads the tree
    /// that [`Input::place`] keeps.
    ///
    /// ```
    /// # #[cfg(feature = "yaml")]
    /// # {
    /// use ctl_input::Input;
    ///
    /// let input = Input::new("page.yaml", "title: A page\nyear: 2026\n");
    /// let entries = input.top_level()?;
    /// let title = &entries[0];
    /// assert_eq!(title.key, "title");
    /// assert_eq!(&input.text()[title.value.clone()], "A page");
    /// assert_eq!(title.string.as_deref(), Some("A page"));
    /// assert_eq!(entries[1].string, None);
    /// # }
    /// # Ok::<(), ctl_input::InputError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the problem and its line, as [`Input::parse`]
    /// gives, for a text its format refuses or that goes past a limit.
    pub fn top_level(&self) -> Result<Vec<Entry>, InputError> {
        let limits = limits::effective(&self.limits);
        self.fits(&limits, Raise::FOR_TEXT)?;
        let read = self
            .kept
            .tree(self.format, &self.text, &limits, Reading::Listing);
        let tree = read
            .as_ref()
            .as_ref()
            .map_err(|refusal| self.refused(refusal, Raise::FOR_TEXT))?;
        Ok(entries(tree, self.format))
    }
}

/// The entries of the mapping `root`, or none when it is no mapping.
fn entries(root: &Node, format: &dyn Format) -> Vec<Entry> {
    let NodeKind::Mapping(pairs) = root.kind() else {
        return Vec::new();
    };
    pairs
        .iter()
        .filter_map(|(key, value)| entry(key, value, format))
        .collect()
}

/// The entry of `key` and `value`, unless the key is no scalar the text
/// wrote: a sequence, a mapping, an alias, or a null written as nothing.
fn entry(key: &Node, value: &Node, format: &dyn Format) -> Option<Entry> {
    let NodeKind::Scalar(name) = key.kind() else {
        return None;
    };
    if key.used_at().is_some() || key.span().is_empty() {
        return None;
    }
    let (range, written) = match value.used_at() {
        Some(alias) => (alias, Written::Alias),
        None => (value.span(), written_as(value)),
    };
    Some(Entry {
        key: name.value().to_owned(),
        value: range,
        written,
        string: string_of(value, format),
    })
}

/// How `value` is written.
fn written_as(value: &Node) -> Written {
    match value.kind() {
        NodeKind::Scalar(scalar) => match scalar.style() {
            ScalarStyle::Plain => Written::Plain,
            ScalarStyle::SingleQuoted => Written::SingleQuoted,
            ScalarStyle::DoubleQuoted => Written::DoubleQuoted,
            ScalarStyle::Literal => Written::Literal,
            ScalarStyle::Folded => Written::Folded,
            ScalarStyle::MultilineBasic => Written::MultilineBasic,
            ScalarStyle::MultilineLiteral => Written::MultilineLiteral,
        },
        NodeKind::Sequence(_) | NodeKind::Mapping(_) => Written::Collection,
    }
}

/// The text of `value` when its format reads it as a string. A TOML datetime
/// is a date, not a string, though its format can read its text.
fn string_of(value: &Node, format: &dyn Format) -> Option<String> {
    let NodeKind::Scalar(scalar) = value.kind() else {
        return None;
    };
    if scalar.syntax() == Some(ScalarSyntax::Datetime) {
        return None;
    }
    match format.resolve(scalar, value.tag(), Wanted::Any) {
        Ok(Resolved::Str(text)) => Some(text.into_owned()),
        Ok(
            Resolved::Null
            | Resolved::Bool(_)
            | Resolved::Int(_)
            | Resolved::Uint(_)
            | Resolved::Float(_),
        )
        | Err(_) => None,
    }
}

//! The top-level entries of a YAML or TOML text, each with the byte range of
//! its value, how the value is written and, for a string, its text.
//!
//! A caller that reads a mapping it did not write, such as the front matter of
//! a Markdown file, learns where each value sits and what it reads as, with no
//! scanner of its own. The format crates decide the types: serde-saphyr for
//! YAML, which resolves anchors, aliases and tags, and the `toml` crate for
//! TOML. The spans come from yamled and from the `toml` tree.

use std::ops::Range;

#[cfg(feature = "input-toml")]
use super::toml_entries;
#[cfg(feature = "input-yaml")]
use super::yaml_entries;
use super::{Format, Input, InputError};

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
    /// The key, as the parser read it: quotes and escapes resolved.
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
    /// An entry whose key is not a string, such as a YAML key that is a
    /// sequence or a number, is left out. A text whose root is no mapping, such
    /// as a sequence or a scalar, has no entries. The text must pass the limits
    /// in force, as [`Input::parse`] holds it to, and a YAML input builds the
    /// document that [`Input::place`] keeps.
    ///
    /// ```
    /// # #[cfg(feature = "input-yaml")]
    /// # {
    /// use ctl_core::input::Input;
    ///
    /// let input = Input::new("page.yaml", "title: A page\nyear: 2026\n");
    /// let entries = input.top_level()?;
    /// let title = &entries[0];
    /// assert_eq!(title.key, "title");
    /// assert_eq!(&input.text()[title.value.clone()], "A page");
    /// assert_eq!(title.string.as_deref(), Some("A page"));
    /// assert_eq!(entries[1].string, None);
    /// # }
    /// # Ok::<(), ctl_core::input::InputError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the parser's problem and its line, as
    /// [`Input::parse`] gives, including a duplicate key, a YAML merge key and
    /// a text past a limit.
    pub fn top_level(&self) -> Result<Vec<Entry>, InputError> {
        match self.format {
            #[cfg(feature = "input-yaml")]
            Format::Yaml => yaml_entries::entries(self),
            #[cfg(feature = "input-toml")]
            Format::Toml => toml_entries::entries(self),
        }
    }
}

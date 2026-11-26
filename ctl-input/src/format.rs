//! The input format interface: what a format reads a text into, and how it
//! reads one scalar for the type a field needs.
//!
//! The built-in formats, [`YAML`] with `yaml` and [`TOML`] with `toml`, and a
//! format a CLI writes are the same kind of thing: a [`Format`] in a
//! [`Formats`](crate::Formats) registry. [`Format::read`] turns text into the
//! [located tree](crate::tree), and [`Format::resolve`] gives each scalar its
//! meaning, so every format keeps its own scalar rules.
#![cfg_attr(
    not(feature = "yaml"),
    doc = "",
    doc = "[`YAML`]: crate#cargo-features"
)]
#![cfg_attr(
    not(feature = "toml"),
    doc = "",
    doc = "[`TOML`]: crate#cargo-features"
)]

use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::ops::Range;

use super::Limits;
use super::messages::{EnglishMessages, Messages, messages};
#[cfg(feature = "toml")]
pub use super::toml_format::{TOML, Toml};
use super::tree::{Node, Scalar, Tag};
#[cfg(feature = "yaml")]
pub use super::yaml_format::{YAML, Yaml};

/// A way to write input, read into the located tree.
///
/// The trait is object safe: [`Formats`](crate::Formats) holds
/// `&'static dyn Format`, as ctl-core's output registry does.
///
/// ```
/// # #[cfg(feature = "yaml")]
/// # {
/// use ctl_input::Limits;
/// use ctl_input::format::{Format, Reading, YAML};
///
/// let formats: Vec<&'static dyn Format> = vec![&YAML];
/// let tree = formats[0].read("a: 1\n", &Limits::NONE, Reading::Strict)?;
/// assert_eq!(tree.span(), 0..4);
/// # }
/// # Ok::<(), ctl_input::format::Refusal>(())
/// ```
pub trait Format: Sync {
    /// The name the registry finds the format by, such as `yaml`.
    fn name(&self) -> &'static str;

    /// The file extensions the format reads, in lower case and without the
    /// dot, such as `yaml` and `yml`.
    fn extensions(&self) -> &'static [&'static str];

    /// Read `text` into the tree, under `limits`.
    ///
    /// The read refuses a second document and anything past a limit, and
    /// `reading` says whether it also refuses duplicate and merge keys. A
    /// format with aliases expands each one while it reads, so the tree holds
    /// no alias node.
    ///
    /// # Errors
    ///
    /// A [`Refusal`] naming the fault and where it is.
    fn read(&self, text: &str, limits: &Limits, reading: Reading) -> Result<Node, Refusal>;

    /// Read one `scalar`, with the `tag` its node carries, as the type a
    /// field `wanted`.
    ///
    /// This applies the rules of a [`Reading::Strict`] read, whichever
    /// reading built the tree: YAML, for one, refuses `.inf` and `.nan` for
    /// [`Wanted::Any`]. A listing asks of each value only whether it is text,
    /// with [`Wanted::Str`].
    ///
    /// # Errors
    ///
    /// A [`Mismatch`] when the scalar cannot be read as `wanted`.
    fn resolve<'a>(
        &self,
        scalar: &'a Scalar,
        tag: Option<&Tag>,
        wanted: Wanted,
    ) -> Result<Resolved<'a>, Mismatch>;
}

/// What a [`Format::read`] refuses besides syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// A config or any text the caller validates: a duplicate key and a merge
    /// key are refused.
    Strict,
    /// A listing of a text the caller did not write, such as the front matter
    /// of a Markdown file: a duplicate key and a merge key stay as ordinary
    /// entries.
    Listing,
}

/// The type a field needs a scalar read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Wanted {
    /// Whatever the scalar is, as a typeless read sees it.
    Any,
    /// A boolean.
    Bool,
    /// An integer.
    Int,
    /// A floating-point number.
    Float,
    /// A string.
    Str,
    /// A null.
    Null,
}

/// A scalar read for the type a field needs.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Resolved<'a> {
    /// A null.
    Null,
    /// A boolean.
    Bool(bool),
    /// An integer that fits `i128`.
    Int(i128),
    /// An integer above `i128::MAX`.
    Uint(u128),
    /// A floating-point number.
    Float(f64),
    /// A string, borrowed from the scalar, or owned when the format decodes
    /// it, as YAML decodes a `!!binary` scalar.
    Str(Cow<'a, str>),
}

/// A scalar the format cannot read as the type a field wanted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    wanted: Wanted,
    detail: Option<String>,
}

impl Mismatch {
    /// A scalar that is not a `wanted`.
    #[must_use]
    pub fn new(wanted: Wanted) -> Self {
        Self {
            wanted,
            detail: None,
        }
    }

    /// This mismatch with the format's own words for why the scalar is not a
    /// `wanted`, such as a YAML float that is not finite.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// The type the field wanted.
    #[must_use]
    pub fn wanted(&self) -> Wanted {
        self.wanted
    }

    /// The format's own words for why the scalar is not a
    /// [`wanted`](Self::wanted), when it gave any. A typeless read has no type
    /// to name, so the Deserializer words its refusal from this.
    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

impl fmt::Display for Mismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.wanted {
            Wanted::Any => "expected a value",
            Wanted::Bool => "expected true or false",
            Wanted::Int => "expected an integer",
            Wanted::Float => "expected a float",
            Wanted::Str => "expected text",
            Wanted::Null => "expected null",
        })
    }
}

impl Error for Mismatch {}

/// Why a [`Format::read`] refused a text, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    fault: Fault,
    span: Option<Range<usize>>,
    defined_at: Option<Range<usize>>,
}

impl Refusal {
    /// A refusal for `fault`, at `span` when the format knows where.
    /// `defined_at` is set when the fault arose in a node an alias replayed:
    /// `span` is then the alias, and `defined_at` where the node is written.
    #[must_use]
    pub fn new(fault: Fault, span: Option<Range<usize>>, defined_at: Option<Range<usize>>) -> Self {
        Self {
            fault,
            span,
            defined_at,
        }
    }

    /// What is wrong.
    #[must_use]
    pub fn fault(&self) -> &Fault {
        &self.fault
    }

    /// The bytes of the text the fault is at, when known. A fault the parser
    /// places only by line and column, such as a YAML scanner error, has an
    /// empty span at that place.
    #[must_use]
    pub fn span(&self) -> Option<Range<usize>> {
        self.span.clone()
    }

    /// Where the node an alias replayed is written, when the fault arose in
    /// one.
    #[must_use]
    pub fn defined_at(&self) -> Option<Range<usize>> {
        self.defined_at.clone()
    }
}

/// The wording [`set_messages`](crate::set_messages) installed, or the English
/// default.
impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&messages().message(&self.fault))
    }
}

impl Error for Refusal {}

/// What a read of a text or of a typed value found wrong.
///
/// Each variant is one problem type, and [`Messages`] words it.
/// A problem in a node an alias replayed is a [`Refusal`] with
/// [`Refusal::defined_at`] set, not a variant of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fault {
    /// The text is not valid in the format.
    Syntax {
        /// What the format's parser says is wrong.
        detail: String,
    },
    /// A key written twice in one mapping, under [`Reading::Strict`].
    DuplicateKey {
        /// The key.
        key: String,
    },
    /// A YAML merge key (`<<`), under [`Reading::Strict`].
    MergeKey,
    /// The text went past a limit.
    Limit {
        /// The [`Limits`] field.
        name: &'static str,
        /// The value in force.
        value: String,
        /// Where a person raises it.
        raise: Raise,
    },
    /// A node of another kind than the type being read needs, such as a
    /// sequence where a struct is read.
    ExpectedKind {
        /// What the type needs, such as `a mapping`.
        expected: String,
        /// What the node is, such as `a sequence`.
        found: String,
    },
    /// A value the type refuses: a scalar the format cannot read as the type
    /// the field needs, a number out of range, or the text of an error the
    /// type's own `Deserialize` raised.
    InvalidValue {
        /// Why the value is refused.
        detail: String,
    },
    /// A key a struct does not have, under `deny_unknown_fields`.
    UnknownField {
        /// The key.
        field: String,
        /// The keys the struct has.
        expected: Vec<&'static str>,
    },
    /// A variant an enum does not have.
    UnknownVariant {
        /// The variant as written.
        variant: String,
        /// The variants the enum has.
        expected: Vec<&'static str>,
    },
    /// A field a struct needs and its mapping lacks.
    MissingField {
        /// The field.
        field: &'static str,
    },
}

/// Where a person raises a limit a text went past.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Raise {
    /// In the repository's file of limits.
    #[cfg(feature = "yaml")]
    InRepository,
    /// In the tool's process default, for the repository's file of limits
    /// itself, which is read before any repository override applies.
    #[cfg(feature = "yaml")]
    InProcess,
    /// Nowhere the user can reach: a build without `yaml` reads no
    /// repository file, so only the tool sets limits.
    #[cfg(not(feature = "yaml"))]
    InTool,
}

impl Fault {
    /// A [`Fault::Limit`] for a text of a CLI's own format that went past
    /// the limit `name` of the value `value`, worded as it is for a text in
    /// memory: where a person raises it is the same as for the built-in
    /// formats.
    #[must_use]
    pub fn limit(name: &'static str, value: impl ToString) -> Self {
        Self::Limit {
            name,
            value: value.to_string(),
            raise: Raise::FOR_TEXT,
        }
    }
}

impl Raise {
    /// Where a limit hit by a CLI's own text is raised.
    #[cfg(feature = "yaml")]
    pub(crate) const FOR_TEXT: Self = Self::InRepository;
    /// Where a limit hit by a CLI's own text is raised.
    #[cfg(not(feature = "yaml"))]
    pub(crate) const FOR_TEXT: Self = Self::InTool;
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&EnglishMessages.message(self))
    }
}

#[cfg(test)]
mod tests {
    use super::{Mismatch, Wanted};

    #[test]
    fn a_mismatch_says_what_was_expected() {
        let expected = [
            (Wanted::Any, "expected a value"),
            (Wanted::Bool, "expected true or false"),
            (Wanted::Int, "expected an integer"),
            (Wanted::Float, "expected a float"),
            (Wanted::Str, "expected text"),
            (Wanted::Null, "expected null"),
        ];
        for (wanted, message) in expected {
            assert_eq!(Mismatch::new(wanted).to_string(), message);
        }
    }
}

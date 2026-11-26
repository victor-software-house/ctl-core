//! The input format interface: what a format reads a text into, and how it
//! reads one scalar for the type a field needs.
//!
//! The built-in formats, [`YAML`] with `yaml` and [`TOML`] with `toml`, and a
//! format a CLI writes are the same kind of thing: a [`Format`] in a
//! [`Formats`](crate::Formats) registry. [`Format::read`] turns text into the
//! [located tree](crate::tree), and [`Format::resolve`] gives each scalar its
//! meaning, so every format keeps its own scalar rules.

use std::error::Error;
use std::fmt;
use std::ops::Range;

use super::Limits;
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
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// A string, borrowed from the scalar.
    Str(&'a str),
}

/// A scalar the format cannot read as the type a field wanted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mismatch {
    wanted: Wanted,
}

impl Mismatch {
    /// A scalar that is not a `wanted`.
    #[must_use]
    pub fn new(wanted: Wanted) -> Self {
        Self { wanted }
    }

    /// The type the field wanted.
    #[must_use]
    pub fn wanted(self) -> Wanted {
        self.wanted
    }
}

impl fmt::Display for Mismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "the scalar is not a {:?}", self.wanted)
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

    /// The bytes of the text the fault is at, when known.
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

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fault.fmt(formatter)
    }
}

impl Error for Refusal {}

/// What a [`Format::read`] found wrong.
///
/// These are the faults of reading a text into the tree. A later release adds
/// the faults of reading a value from it.
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
    },
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { detail } => formatter.write_str(detail),
            Self::DuplicateKey { key } => write!(formatter, "duplicate key `{key}`"),
            Self::MergeKey => formatter.write_str("merge keys are not allowed"),
            Self::Limit { name, value } => write!(formatter, "exceeds {name} ({value})"),
        }
    }
}

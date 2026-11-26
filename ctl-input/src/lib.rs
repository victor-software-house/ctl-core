//! Declared input: read a file, parse it, validate it once, and report every
//! problem on its file and line.
//!
//! Each format is its own feature. `yaml` parses YAML with `serde-saphyr`,
//! and `toml` parses TOML with the `toml` crate; neither needs the other.
//! `validate` adds `Input::check` and `Input::load`, which validate with
//! garde. This is the one boundary where a CLI's config enters. Downstream
//! code keeps the validated type and never checks it again. With no format
//! feature the crate is empty.
//!
//! A check that runs after the read, such as one that needs a file the config
//! names, places its problem with a [`FieldPath`] and [`Input::problem_at`],
//! the way the loader places garde's.
//!
//! A text's [`Format`] is chosen, never guessed. [`Input::open`] takes it from
//! the file's extension through the process's [`Formats`] registry,
//! [`Formats::open`] through a registry of the caller's, and
//! [`Input::new_with_format`] takes it for text in memory. With `yaml`,
//! `Input::new` and `Input::read` are YAML.
//!
//! ## The tree, the format trait and the registry
//!
//! `tree` holds the located tree: each node keeps its byte span, each scalar
//! its processed value and style, and each mapping its key and value nodes.
//! `format::Format` is the trait that reads a text into that tree and
//! resolves one scalar for the type a field needs, and `Formats` is the
//! registry a CLI adds its own format to. `YAML` and `TOML` are the two
//! built-in formats, each a value of its feature.
//!
//! An [`Input`] reads its text into that tree once and keeps it. Parsing,
//! placing a problem, listing the top-level entries and checking with garde
//! all work on it, so every format, a CLI's own included, is read, placed and
//! worded the same way.
//!
//! ## The Deserializer, the messages and `Tagged`
//!
//! `de::deserialize` reads a typed value from a tree, asking the format to
//! resolve each scalar for the type its field needs.
//! A problem is a `format::Fault` with the bytes it sits at, and
//! `Messages` words it, in English by default and in another voice after
//! `set_messages`.
//! `Tagged` reads a node's tag beside its value.
//!
//! ```
//! # #[cfg(all(feature = "yaml", feature = "validate"))]
//! # {
//! use ctl_input::Input;
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize)]
//! struct Config {
//!     name: String,
//! }
//!
//! let config: Config = Input::new("toy.yaml", "name: release\n").parse()?;
//! assert_eq!(config.name, "release");
//!
//! let error = Input::new("toy.yaml", "name: a\nname: b\n")
//!     .parse::<Config>()
//!     .unwrap_err();
//! assert!(error.to_string().starts_with("toy.yaml"));
//! # }
//! # Ok::<(), ctl_input::InputError>(())
//! ```
//!
//! ## Cargo features
#![cfg_attr(
    not(any(feature = "yaml", feature = "toml")),
    doc = "",
    doc = "[`FieldPath`]: crate#cargo-features",
    doc = "[`Input::problem_at`]: crate#cargo-features",
    doc = "[`Format`]: crate#cargo-features",
    doc = "[`Formats`]: crate#cargo-features",
    doc = "[`Formats::open`]: crate#cargo-features",
    doc = "[`Input`]: crate#cargo-features",
    doc = "[`Input::open`]: crate#cargo-features",
    doc = "[`Input::new_with_format`]: crate#cargo-features"
)]
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg(any(feature = "yaml", feature = "toml"))]

use std::fmt;
use std::path::Path;

#[cfg(feature = "validate")]
use garde::Validate;
use serde::de::DeserializeOwned;

pub mod de;
mod entries;
mod exceeded;
mod field_path;
pub mod format;
mod formats;
mod input_error;
mod kept;
mod limits;
mod messages;
mod placing;
#[cfg(feature = "toml")]
mod toml_format;
pub mod tree;
#[cfg(feature = "yaml")]
mod yaml_format;

pub use de::Tagged;
pub use entries::{Entry, Written};
pub use field_path::{FieldPath, Place, Step};
pub use format::Format;
#[cfg(feature = "toml")]
pub use format::TOML;
#[cfg(feature = "yaml")]
pub use format::YAML;
use format::{Fault, Raise, Reading, Refusal};
pub use formats::{Formats, set_default_formats};
use input_error::read_text;
pub use input_error::{InputError, Problem};
use kept::Kept;
pub use limits::{Limits, set_default_limits};
#[cfg(feature = "yaml")]
pub use limits::{Loosened, REPO_FILE, use_repo_limits};
use messages::messages;
pub use messages::{EnglishMessages, Messages, set_messages};
use placing::At;

/// A named text, ready to parse and validate.
///
/// The name is usually the file's path, and appears in every problem. Build
/// one with [`Input::new_with_format`] for text in memory, or [`Input::open`]
/// for a file. With `yaml`, `Input::new` and `Input::read` build a YAML
/// one.
#[derive(Clone)]
pub struct Input {
    name: String,
    text: String,
    lines_before: usize,
    limits: Limits,
    format: &'static dyn Format,
    /// The schema fields serde flattens into their parents, which
    /// [`Input::flattened`] declares.
    #[cfg(feature = "validate")]
    flattened: Vec<FieldPath>,
    /// The tree the text was read into, for every operation to share.
    kept: Kept,
}

/// A format has no `Debug`, so it shows by its name, and the kept tree is
/// left out.
impl fmt::Debug for Input {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut shown = formatter.debug_struct("Input");
        shown
            .field("name", &self.name)
            .field("text", &self.text)
            .field("lines_before", &self.lines_before)
            .field("limits", &self.limits)
            .field("format", &self.format.name());
        #[cfg(feature = "validate")]
        shown.field("flattened", &self.flattened);
        shown.finish_non_exhaustive()
    }
}

/// Two inputs are equal when they read the same text the same way. A format
/// is compared by its name, and the kept tree is derived from the rest, so
/// placing on one input does not make it differ from its clone.
impl PartialEq for Input {
    fn eq(&self, other: &Self) -> bool {
        let same = self.name == other.name
            && self.text == other.text
            && self.lines_before == other.lines_before
            && self.limits == other.limits
            && self.format.name() == other.format.name();
        #[cfg(feature = "validate")]
        let same = same && self.flattened == other.flattened;
        same
    }
}

impl Eq for Input {}

impl Input {
    /// A YAML input from text already in memory, whatever the extension of
    /// `name`. Set another format with [`Input::format`].
    #[cfg(feature = "yaml")]
    #[must_use]
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new_with_format(name, text, &YAML)
    }

    /// An input of `format` from text already in memory, whatever the
    /// extension of `name`. The format is any [`Format`], one a CLI wrote
    /// included, and a build needs only its own feature: a build without YAML
    /// builds text in memory with it.
    #[must_use]
    pub fn new_with_format(
        name: impl Into<String>,
        text: impl Into<String>,
        format: &'static dyn Format,
    ) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
            lines_before: 0,
            limits: Limits::NONE,
            format,
            #[cfg(feature = "validate")]
            flattened: Vec::new(),
            kept: Kept::default(),
        }
    }

    /// Read a YAML file, named by its path, whatever its extension.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the I/O failure.
    #[cfg(feature = "yaml")]
    pub fn read(path: &Path) -> Result<Self, InputError> {
        let (name, text) = read_text(path)?;
        Ok(Self::new(name, text))
    }

    /// Read a file and take its [`Format`] from its extension, through the
    /// registry the process uses: what [`set_default_formats`] installed, or
    /// else [`Formats::builtin`]. [`Formats::open`] reads through a registry
    /// of the caller's instead.
    ///
    /// # Errors
    ///
    /// An [`InputError`] for an extension [`Formats::of`] refuses, which is
    /// checked before the file is read, or with the I/O failure.
    pub fn open(path: &Path) -> Result<Self, InputError> {
        formats::default_formats().open(path)
    }

    /// Set how the text is written, over what the constructor took.
    #[must_use]
    pub fn format(mut self, format: &'static dyn Format) -> Self {
        self.format = format;
        self.kept = Kept::default();
        self
    }

    /// The name given at construction, usually the file's path.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The text as read, unchanged.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Split YAML frontmatter from a Markdown body.
    ///
    /// The frontmatter is the text between a first line of `---` and the next
    /// line of `---`. It is YAML whatever this input's format, and problems in
    /// it keep the line numbers of the whole file. The second item is the body
    /// after the closing line. Returns `None` when the text does not start with
    /// a `---` line, or never closes it.
    #[cfg(feature = "yaml")]
    #[must_use]
    pub fn frontmatter(&self) -> Option<(Self, &str)> {
        let text = self.text.strip_prefix('\u{feff}').unwrap_or(&self.text);
        let rest = text
            .strip_prefix("---\n")
            .or_else(|| text.strip_prefix("---\r\n"))?;
        let mut offset = 0;
        for line in rest.split_inclusive('\n') {
            if line.trim_end() == "---" {
                let matter = Self {
                    name: self.name.clone(),
                    text: rest[..offset].to_owned(),
                    lines_before: self.lines_before + 1,
                    limits: self.limits.clone(),
                    format: &YAML,
                    #[cfg(feature = "validate")]
                    flattened: self.flattened.clone(),
                    kept: Kept::default(),
                };
                return Some((matter, &rest[offset + line.len()..]));
            }
            offset += line.len();
        }
        None
    }

    /// Override limits for this input alone, above every other tier. See
    /// [`Limits`] for the order.
    #[must_use]
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Parse the text into `T`, without validating it.
    ///
    /// The text is read into the input's tree, which the input keeps for
    /// placing and listing, and `T` is read from that tree. The first problem
    /// found is the one reported, in the order the text is written. The format
    /// decides what a scalar can be: in YAML only `true` and `false` are
    /// booleans, and a TOML string fills only a string. A duplicate key or a
    /// merge key (`<<`) is refused.
    ///
    /// The text must fit the limits in force, decided field by field through
    /// the tiers [`Limits`] lists. TOML reads only `max_reader_input_bytes`,
    /// `max_depth`, and `max_nodes`, because the other fields name YAML
    /// syntax.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the problem and its line. A limit problem names
    /// the limit, its value, and where to raise it, or, without `yaml`, that
    /// the tool sets it.
    pub fn parse<T: DeserializeOwned>(&self) -> Result<T, InputError> {
        self.parse_within(&limits::effective(&self.limits), Raise::FOR_TEXT)
    }

    /// [`Self::parse`] under exactly `limits`, whatever the tiers say.
    /// `raise` is where a person raises a limit the text goes past.
    fn parse_within<T: DeserializeOwned>(
        &self,
        limits: &Limits,
        raise: Raise,
    ) -> Result<T, InputError> {
        self.fits(limits, raise)?;
        let read = self
            .kept
            .tree(self.format, &self.text, limits, Reading::Strict);
        let tree = read
            .as_ref()
            .as_ref()
            .map_err(|refusal| self.refused(refusal, raise))?;
        de::deserialize(tree, self.format).map_err(|refusal| self.refused(&refusal, raise))
    }

    /// The input's problem for `refusal`, placed on its line and column, and
    /// worded by [`Messages`]. A refusal in a node an alias replayed also
    /// names where its value is defined, counted from the file.
    fn refused(&self, refusal: &Refusal, raise: Raise) -> InputError {
        let fault = if let Fault::Limit { name, value, .. } = refusal.fault().clone() {
            Fault::Limit { name, value, raise }
        } else {
            refusal.fault().clone()
        };
        let mut message = worded(&fault);
        if let Some(defined) = refusal.defined_at() {
            let (line, column) = tree::position(&self.text, defined.start);
            let line = line + self.lines_before;
            message = format!("{message} (defined at line {line}, column {column})");
        }
        let place = refusal
            .span()
            .map(|span| tree::position(&self.text, span.start));
        InputError::one(
            self.name.clone(),
            Problem {
                line: place.map(|(line, _)| line + self.lines_before),
                column: place.map(|(_, column)| column),
                field: String::new(),
                message,
            },
        )
    }

    /// Whether the text is within the size `limits` allow.
    fn fits(&self, limits: &Limits, raise: Raise) -> Result<(), InputError> {
        let bounds = limits.bounds();
        if self.text.len() > bounds.input_bytes {
            let fault = Fault::TooLarge {
                bytes: u64::try_from(self.text.len()).unwrap_or(u64::MAX),
                limit: u64::try_from(bounds.input_bytes).unwrap_or(u64::MAX),
                raise,
            };
            return Err(InputError::whole(self.name.clone(), worded(&fault)));
        }
        Ok(())
    }

    /// Parse the text into `T`, then validate it with `T`'s default context.
    ///
    /// # Errors
    ///
    /// As [`Input::parse`] and [`Input::check`].
    #[cfg(feature = "validate")]
    pub fn load<T>(&self) -> Result<T, InputError>
    where
        T: DeserializeOwned + Validate,
        T::Context: Default,
    {
        let value = self.parse::<T>()?;
        self.check(&value, &T::Context::default())?;
        Ok(value)
    }
}

/// The words of `fault` from the process's [`Messages`], with each control
/// character escaped so a name or a value the text wrote cannot move a
/// terminal that prints the problem.
fn worded(fault: &Fault) -> String {
    messages::printable(&messages().message(fault))
}

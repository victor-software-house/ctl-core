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

use std::error::Error;
use std::path::Path;
use std::{fmt, fs, io, slice};

#[cfg(feature = "validate")]
use garde::Validate;
use serde::de::DeserializeOwned;

pub mod de;
mod entries;
mod exceeded;
mod field_path;
pub mod format;
mod formats;
mod kept;
mod limits;
mod messages;
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
use kept::Kept;
pub use limits::{Limits, set_default_limits};
#[cfg(feature = "yaml")]
pub use limits::{Loosened, REPO_FILE, use_repo_limits};
use messages::messages;
pub use messages::{EnglishMessages, Messages, set_messages};

/// What a placing answers with for a key of the path: where its value starts,
/// or where the key itself is written.
#[derive(Clone, Copy)]
enum At {
    Value,
    Key,
}

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

/// Why an input was refused: every problem found, each placed on a line when
/// the text allows it.
///
/// `Display` prints one problem per line as `name:line:column: field: message`,
/// leaving out what is not known.
///
/// A file that could not be read keeps its [`io::ErrorKind`], which
/// [`Self::io_failure`] returns. So a caller can treat a missing file apart
/// from a refused one. A field can join this type in a minor release.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct InputError {
    /// The input's name, usually its path.
    pub name: String,
    /// What is wrong, in the order found.
    pub problems: Vec<Problem>,
    /// The I/O failure, for an input that could not be read.
    io_failure: Option<io::ErrorKind>,
}

/// One thing wrong with an input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// 1-based line, when known.
    pub line: Option<usize>,
    /// 1-based column, when known.
    pub column: Option<usize>,
    /// The field it concerns, as garde names it (`packages[1].name`); empty
    /// for a problem with the whole text.
    pub field: String,
    /// What a person changes to fix it.
    pub message: String,
}

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

    /// Declare that the schema field at `field` is flattened into its parent
    /// with `#[serde(flatten)]`, so the file writes its entries directly under
    /// the parent.
    ///
    /// garde names the field in each problem's path all the same, and serde
    /// gives no sign of the flattening at run time, so the schema declares it.
    /// [`Input::check`] then drops the field's step from each garde path whose
    /// leading steps are `field`, before it places the problem and prints its
    /// field. `field` is written in the schema's terms, the way garde names it,
    /// and each call adds one field. A declaration that matches no garde path
    /// changes nothing. [`Input::place`], [`Input::place_key`], and
    /// [`Input::problem_at`] take paths in the file's terms, so a declaration
    /// does not touch them.
    ///
    /// # Examples
    ///
    /// verctl's `assets` holds its targets in a flattened map field `targets`.
    /// Without the declaration, the problem prints as
    /// `assets.targets.linux-x64.files` and sits on the line below `assets:`.
    ///
    /// ```
    /// # #[cfg(all(feature = "yaml", feature = "validate"))]
    /// # {
    /// use std::collections::BTreeMap;
    ///
    /// use ctl_input::{FieldPath, Input};
    /// use garde::Validate;
    /// use serde::Deserialize;
    ///
    /// #[derive(Debug, Deserialize, Validate)]
    /// struct Config {
    ///     #[garde(dive)]
    ///     assets: Assets,
    /// }
    ///
    /// #[derive(Debug, Deserialize, Validate)]
    /// struct Assets {
    ///     #[serde(flatten)]
    ///     #[garde(dive)]
    ///     targets: BTreeMap<String, Target>,
    /// }
    ///
    /// #[derive(Debug, Deserialize, Validate)]
    /// struct Target {
    ///     #[garde(custom(at_least_one_file))]
    ///     files: Vec<String>,
    /// }
    ///
    /// fn at_least_one_file(files: &[String], _: &()) -> garde::Result {
    ///     if files.is_empty() {
    ///         return Err(garde::Error::new("must declare at least one file"));
    ///     }
    ///     Ok(())
    /// }
    ///
    /// let error = Input::new("ver.yaml", "assets:\n  linux-x64:\n    files: []\n")
    ///     .flattened(FieldPath::root().key("assets").key("targets"))
    ///     .load::<Config>()
    ///     .unwrap_err();
    /// assert_eq!(
    ///     error.to_string(),
    ///     "ver.yaml:3:12: assets.linux-x64.files: must declare at least one file"
    /// );
    /// # }
    /// ```
    #[cfg(feature = "validate")]
    #[must_use]
    pub fn flattened(mut self, field: FieldPath) -> Self {
        self.flattened.push(field);
        self
    }

    /// `path` without the step of each declared flattened field it passes
    /// through, or `None` when it passes through none.
    #[cfg(feature = "validate")]
    fn unflatten(&self, path: &FieldPath) -> Option<FieldPath> {
        let positions: Vec<usize> = self
            .flattened
            .iter()
            .filter(|field| path.starts_with(field))
            .filter_map(|field| field.steps().len().checked_sub(1))
            .collect();
        (!positions.is_empty()).then(|| path.without(&positions))
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

    /// Validate a parsed `value` against `context`.
    ///
    /// Each of garde's problems lands on the line of the field it names, or
    /// of its nearest written ancestor. The place comes from the parts of
    /// garde's path through [`Input::place`], so a key that holds a `.` is
    /// found as one key. `Problem::field` is garde's printed path, for display
    /// only. A path through a field [`Input::flattened`] declares loses that
    /// field's step, and its `Problem::field` is the shorter path as
    /// [`FieldPath`] prints it.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with every problem garde reports.
    #[cfg(feature = "validate")]
    pub fn check<T: Validate + ?Sized>(
        &self,
        value: &T,
        context: &T::Context,
    ) -> Result<(), InputError> {
        let Err(report) = value.validate_with(context) else {
            return Ok(());
        };
        let reported: Vec<_> = report
            .iter()
            .map(|(path, error)| {
                let steps = FieldPath::from(path);
                let (field, steps) = match self.unflatten(&steps) {
                    Some(kept) => (kept.to_string(), kept),
                    None => (path.to_string(), steps),
                };
                (field, steps, error.message().to_owned())
            })
            .collect();
        let paths: Vec<_> = reported.iter().map(|(_, path, _)| path.clone()).collect();
        let places = self.places(&paths, At::Value);
        let problems = reported
            .into_iter()
            .zip(places)
            .map(|((field, _, message), place)| Problem {
                line: place.map(|at| at.line),
                column: place.map(|at| at.column),
                field,
                message,
            });
        InputError::try_new(self.name.clone(), problems).map_or(Ok(()), Err)
    }

    /// Where `path` is written in the text, or where its nearest written
    /// ancestor is.
    ///
    /// The line counts the whole file, so a frontmatter input adds the lines
    /// before it. A path the text lacks gives the place of the last step that
    /// exists, and a caller that needs an exact hit compares the result with
    /// the place of the path's parent. `None` when the text does not parse.
    ///
    /// `None` too when the text goes past the limits in force, as
    /// [`Input::parse`] would refuse it, so placing never reads a text the
    /// limits keep out.
    ///
    /// The first placing reads the text into the tree and keeps it, so
    /// [`Input::parse`], [`Input::check`], [`Input::top_level`] and later calls
    /// share one read, for every format. A repeated key or a merge key does not
    /// stop placing, though [`Input::parse`] refuses it.
    ///
    /// ```
    /// # #[cfg(feature = "yaml")]
    /// # {
    /// use ctl_input::{FieldPath, Input};
    ///
    /// let input = Input::new("app.yaml", "files:\n  'docs/**/*.md':\n    label: ''\n");
    /// let path = FieldPath::root()
    ///     .key("files")
    ///     .key("docs/**/*.md")
    ///     .key("label");
    /// let at = input.place(&path).map(|at| (at.line, at.column));
    /// assert_eq!(at, Some((3, 12)));
    /// # }
    /// ```
    #[must_use]
    pub fn place(&self, path: &FieldPath) -> Option<Place> {
        self.places(slice::from_ref(path), At::Value)
            .pop()
            .flatten()
    }

    /// Where the last step of `path` is written as a key, or where its nearest
    /// written ancestor's key is.
    ///
    /// [`Input::place`] gives where a value starts. For a scalar or a flow
    /// value that is the line of the key, and for a block mapping or a block
    /// list it is the first line below it. A fault about the key itself, such
    /// as a key that names nothing, belongs on the key, and this finds it.
    ///
    /// A step that is an index has no key, and neither has the root, so they
    /// give the place of their value, as `place` does. A path the text lacks
    /// gives the key of the last step that exists. The line counts the whole
    /// file, `None` is for a text that does not parse or goes past the limits,
    /// and the tree is read once and shared with [`Input::place`].
    ///
    /// ```
    /// # #[cfg(feature = "yaml")]
    /// # {
    /// use ctl_input::{FieldPath, Input};
    ///
    /// let input = Input::new("app.yaml", "files:\n  'docs/**/*.md':\n    label: ''\n");
    /// let glob = FieldPath::root().key("files").key("docs/**/*.md");
    /// let value = input.place(&glob).map(|at| (at.line, at.column));
    /// let key = input.place_key(&glob).map(|at| (at.line, at.column));
    /// assert_eq!(value, Some((3, 5)));
    /// assert_eq!(key, Some((2, 3)));
    /// # }
    /// ```
    #[must_use]
    pub fn place_key(&self, path: &FieldPath) -> Option<Place> {
        self.places(slice::from_ref(path), At::Key).pop().flatten()
    }

    /// A [`Problem`] for `path` with `message`, placed as [`Input::place`]
    /// places it, with the path printed as garde prints a field.
    ///
    /// A CLI uses it for a check that runs after the read, such as one that
    /// needs a file the config names.
    #[must_use]
    pub fn problem_at(&self, path: &FieldPath, message: impl Into<String>) -> Problem {
        let place = self.place(path);
        Problem {
            line: place.map(|at| at.line),
            column: place.map(|at| at.column),
            field: path.to_string(),
            message: message.into(),
        }
    }

    /// The place of each path, with the frontmatter lines added.
    fn places(&self, paths: &[FieldPath], at: At) -> Vec<Option<Place>> {
        let limits = limits::effective(&self.limits);
        if self.text.len() > limits.bounds().input_bytes {
            return vec![None; paths.len()];
        }
        let read = self
            .kept
            .tree(self.format, &self.text, &limits, Reading::Listing);
        let Ok(tree) = read.as_ref() else {
            return vec![None; paths.len()];
        };
        paths
            .iter()
            .map(|path| {
                let (line, column) =
                    tree::position(&self.text, tree.nearest(path.steps(), at).start);
                Some(Place::new(line + self.lines_before, column))
            })
            .collect()
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

/// The file at `path`, named by its path, or the I/O failure.
fn read_text(path: &Path) -> Result<(String, String), InputError> {
    let name = path.display().to_string();
    match fs::read_to_string(path) {
        Ok(text) => Ok((name, text)),
        Err(error) => Err(InputError::unread(name, &error)),
    }
}

impl InputError {
    /// The problems a CLI found in the input `name`, in the order given, so
    /// it reports its own refusals as ctl-core reports garde's. Each
    /// [`Problem`] is placed with [`Input::place`] or [`Input::problem_at`],
    /// or carries no line.
    ///
    /// Returns `None` when `problems` is empty, since an error with no problem
    /// would print nothing. So a CLI hands it every check's findings and
    /// refuses the input only when one turned up.
    ///
    /// # Examples
    ///
    /// ```
    /// use ctl_input::{InputError, Problem};
    ///
    /// let refused = InputError::try_new(
    ///     "compose.yaml",
    ///     [Problem {
    ///         line: Some(4),
    ///         column: Some(5),
    ///         field: "services.dev.privileged".to_owned(),
    ///         message: "this host runs no privileged service".to_owned(),
    ///     }],
    /// );
    /// assert_eq!(
    ///     refused.map(|error| error.to_string()).as_deref(),
    ///     Some("compose.yaml:4:5: services.dev.privileged: this host runs no privileged service")
    /// );
    /// assert_eq!(InputError::try_new("compose.yaml", []), None);
    /// ```
    #[must_use]
    pub fn try_new(
        name: impl Into<String>,
        problems: impl IntoIterator<Item = Problem>,
    ) -> Option<Self> {
        let problems = problems.into_iter().collect::<Vec<_>>();
        (!problems.is_empty()).then(|| Self {
            name: name.into(),
            problems,
            io_failure: None,
        })
    }

    /// A refusal of the whole input `name`, on no line and no field, such as
    /// a value that only makes sense once the file is read whole.
    ///
    /// # Examples
    ///
    /// ```
    /// use ctl_input::InputError;
    ///
    /// let error = InputError::whole("compose.yaml", "the file names no service");
    /// assert_eq!(error.to_string(), "compose.yaml: the file names no service");
    /// ```
    #[must_use]
    pub fn whole(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::one(
            name.into(),
            Problem {
                line: None,
                column: None,
                field: String::new(),
                message: message.into(),
            },
        )
    }

    /// The one problem ctl-core found in the input `name`, with no I/O
    /// failure.
    pub(crate) fn one(name: String, problem: Problem) -> Self {
        Self {
            name,
            problems: vec![problem],
            io_failure: None,
        }
    }

    /// The input `name` could not be read: the failure's text is the one
    /// problem, and its `io::ErrorKind` is kept.
    fn unread(name: String, error: &io::Error) -> Self {
        Self {
            io_failure: Some(error.kind()),
            ..Self::whole(name, error.to_string())
        }
    }

    /// The I/O failure when the file could not be read, such as
    /// [`io::ErrorKind::NotFound`]. `None` when the text was read and refused.
    #[must_use]
    pub const fn io_failure(&self) -> Option<io::ErrorKind> {
        self.io_failure
    }
}

/// One problem per line: `name:line:column: field: message`, leaving out
/// what is not known.
impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, problem) in self.problems.iter().enumerate() {
            if index > 0 {
                writeln!(formatter)?;
            }
            write!(formatter, "{}", self.name)?;
            if let Some(line) = problem.line {
                write!(formatter, ":{line}")?;
                if let Some(column) = problem.column {
                    write!(formatter, ":{column}")?;
                }
            }
            if problem.field.is_empty() {
                write!(formatter, ": {}", problem.message)?;
            } else {
                write!(formatter, ": {}: {}", problem.field, problem.message)?;
            }
        }
        Ok(())
    }
}

impl Error for InputError {}

//! Declared input: read a file, parse it, validate it once, and report every
//! problem on its file and line.
//!
//! Each format is its own feature. `input-yaml` parses YAML with
//! `serde-saphyr`, and `input-toml` parses TOML with the `toml` crate; neither
//! needs the other. `validate` adds `Input::check` and `Input::load`, which
//! validate with garde, and `input` is `input-yaml` with `validate`. This is
//! the one boundary where a CLI's config enters. Downstream code keeps the
//! validated type and never checks it again.
//!
//! A check that runs after the read, such as one that needs a file the config
//! names, places its problem with a [`FieldPath`] and [`Input::problem_at`],
//! the way the loader places garde's.
//!
//! A text's [`Format`] is chosen, never guessed. [`Input::open`] reads it from
//! the file's extension, and [`Input::new_with_format`] takes it for text in
//! memory. With `input-yaml`, `Input::new` and `Input::read` are YAML.
//!
//! ```
//! # #[cfg(feature = "input")]
//! # {
//! use ctl_core::input::Input;
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
//! # Ok::<(), ctl_core::input::InputError>(())
//! ```

use std::path::Path;
use std::{fmt, io};

#[cfg(feature = "validate")]
use garde::Validate;
use serde::de::DeserializeOwned;

mod entries;
mod field_path;
mod limits;
#[cfg(feature = "input-toml")]
mod toml_entries;
#[cfg(feature = "input-toml")]
mod toml_text;
#[cfg(feature = "input-yaml")]
mod yaml_entries;
#[cfg(feature = "input-yaml")]
mod yaml_text;

pub use entries::{Entry, Written};
pub use field_path::{FieldPath, Place, Step};
use limits::{Exceeded, Raise};
pub use limits::{Limits, set_default_limits};
#[cfg(feature = "input-yaml")]
pub use limits::{Loosened, REPO_FILE, use_repo_limits};
/// A value and the YAML tag written on its node, such as `!override` in
/// `code_span_size: !override 0`: `Tagged(0, Some("!override".into()))`.
/// A node with no tag reads as `Tagged(value, None)`, and an untagged enum
/// inside reads the plain value of a tagged node.
#[cfg(feature = "input-yaml")]
pub use serde_saphyr::Tagged;

/// What a placing answers with for a key of the path: where its value starts,
/// or where the key itself is written.
#[derive(Clone, Copy)]
enum At {
    Value,
    Key,
}

/// How a text is written.
///
/// Non-exhaustive: each variant exists only with its format's feature.
/// Cargo unifies features across a build, so a caller's `match` needs a
/// wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    /// YAML, read by serde-saphyr and placed by yamled.
    #[cfg(feature = "input-yaml")]
    Yaml,
    /// TOML, read and placed by the `toml` crate.
    #[cfg(feature = "input-toml")]
    Toml,
}

impl Format {
    /// The format a file's extension names, compared without regard to case:
    /// `.toml` is TOML, and `.yaml` and `.yml` are YAML.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with no line for any other extension or none, naming
    /// the extension and the accepted ones. One naming the feature for an
    /// extension whose format's feature is off.
    pub fn of(path: &Path) -> Result<Self, InputError> {
        let name = path.display().to_string();
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
        match extension.as_deref() {
            #[cfg(feature = "input-yaml")]
            Some("yaml" | "yml") => Ok(Self::Yaml),
            #[cfg(not(feature = "input-yaml"))]
            Some(yaml @ ("yaml" | "yml")) => Err(InputError::whole(
                name,
                format!("`.{yaml}` needs ctl-core's input-yaml feature; {ACCEPTED}"),
            )),
            #[cfg(feature = "input-toml")]
            Some("toml") => Ok(Self::Toml),
            #[cfg(not(feature = "input-toml"))]
            Some("toml") => Err(InputError::whole(
                name,
                format!("`.toml` needs ctl-core's input-toml feature; {ACCEPTED}"),
            )),
            Some(other) => Err(InputError::whole(
                name,
                format!("unsupported extension .{other}; {ACCEPTED}"),
            )),
            None => Err(InputError::whole(
                name,
                format!("no file extension; {ACCEPTED}"),
            )),
        }
    }
}

/// What a refusal tells a person to name the file: the extensions this build
/// reads.
#[cfg(all(feature = "input-yaml", feature = "input-toml"))]
const ACCEPTED: &str = "name the file *.toml, *.yaml, or *.yml";
#[cfg(all(feature = "input-yaml", not(feature = "input-toml")))]
const ACCEPTED: &str = "name the file *.yaml or *.yml";
#[cfg(all(feature = "input-toml", not(feature = "input-yaml")))]
const ACCEPTED: &str = "name the file *.toml";

/// A named text, ready to parse and validate.
///
/// The name is usually the file's path, and appears in every problem. Build
/// one with [`Input::new_with_format`] for text in memory, or [`Input::open`]
/// for a file. With `input-yaml`, `Input::new` and `Input::read` build a YAML
/// one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    name: String,
    text: String,
    lines_before: usize,
    limits: Limits,
    format: Format,
    /// The schema fields serde flattens into their parents, which
    /// [`Input::flattened`] declares.
    #[cfg(feature = "validate")]
    flattened: Vec<FieldPath>,
    /// The YAML document that placing reads, built on first use.
    #[cfg(feature = "input-yaml")]
    yaml_document: yaml_text::Located,
}

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
    #[cfg(feature = "input-yaml")]
    #[must_use]
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new_with_format(name, text, Format::Yaml)
    }

    /// An input of `format` from text already in memory, whatever the
    /// extension of `name`. It needs only `format`'s feature, so a build
    /// without YAML builds text in memory with it.
    #[must_use]
    pub fn new_with_format(
        name: impl Into<String>,
        text: impl Into<String>,
        format: Format,
    ) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
            lines_before: 0,
            limits: Limits::NONE,
            format,
            #[cfg(feature = "validate")]
            flattened: Vec::new(),
            #[cfg(feature = "input-yaml")]
            yaml_document: yaml_text::Located::default(),
        }
    }

    /// Read a YAML file, named by its path, whatever its extension.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the I/O failure.
    #[cfg(feature = "input-yaml")]
    pub fn read(path: &Path) -> Result<Self, InputError> {
        let (name, text) = read_text(path)?;
        Ok(Self::new(name, text))
    }

    /// Read a file and take its [`Format`] from its extension, as
    /// [`Format::of`] does. The extension is checked before the file is read.
    ///
    /// # Errors
    ///
    /// An [`InputError`] for an extension [`Format::of`] refuses, or with the
    /// I/O failure.
    pub fn open(path: &Path) -> Result<Self, InputError> {
        let format = Format::of(path)?;
        let (name, text) = read_text(path)?;
        Ok(Self::new_with_format(name, text, format))
    }

    /// Set how the text is written, over what the constructor took.
    #[must_use]
    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
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
    #[cfg(feature = "input-yaml")]
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
                    format: Format::Yaml,
                    #[cfg(feature = "validate")]
                    flattened: self.flattened.clone(),
                    yaml_document: yaml_text::Located::default(),
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
    /// # #[cfg(feature = "input")]
    /// # {
    /// use std::collections::BTreeMap;
    ///
    /// use ctl_core::input::{FieldPath, Input};
    /// use ctl_core::validate::at_least_one;
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
    ///     #[garde(custom(at_least_one("file")))]
    ///     files: Vec<String>,
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
    /// In YAML only `true` and `false` are booleans, and a duplicate key or a
    /// merge key (`<<`) is refused. In TOML the format itself refuses a
    /// duplicate key.
    ///
    /// The text must fit the limits in force, decided field by field through
    /// the tiers [`Limits`] lists. TOML reads only `max_reader_input_bytes`,
    /// `max_depth`, and `max_nodes`, because the other fields name YAML
    /// syntax.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the parser's problem and its line. A limit
    /// problem names the limit, its value, and where to raise it, or, without
    /// `input-yaml`, that the tool sets it.
    pub fn parse<T: DeserializeOwned>(&self) -> Result<T, InputError> {
        self.parse_within(&limits::effective(&self.limits), Raise::FOR_TEXT)
    }

    /// [`Self::parse`] under exactly `limits`, whatever the tiers say.
    fn parse_within<T: DeserializeOwned>(
        &self,
        limits: &Limits,
        raise: Raise,
    ) -> Result<T, InputError> {
        self.fits(limits, raise)?;
        match self.format {
            #[cfg(feature = "input-yaml")]
            Format::Yaml => yaml_text::parse(self, &limits.budget(), raise),
            #[cfg(feature = "input-toml")]
            Format::Toml => toml_text::parse(self, limits.bounds(), raise),
        }
    }

    /// Whether the text is within the size `limits` allow.
    fn fits(&self, limits: &Limits, raise: Raise) -> Result<(), InputError> {
        let bounds = limits.bounds();
        if self.text.len() > bounds.input_bytes {
            return Err(InputError::whole(
                self.name.clone(),
                format!(
                    "is {} bytes, which {}",
                    self.text.len(),
                    Exceeded::input_bytes(bounds.input_bytes, raise)
                ),
            ));
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
            })
            .collect();
        Err(InputError::of(self.name.clone(), problems))
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
    /// [`Input::parse`] would refuse it, so placing never parses a text the
    /// limits keep out. A YAML input checks `max_reader_input_bytes`,
    /// `max_depth` and `max_nodes` before it hands the text to yamled, which
    /// has no budget. A TOML input checks `max_reader_input_bytes`, and
    /// the parse of its tree is the one [`Input::parse`] does first.
    ///
    /// A YAML input parses its text for the first placing and keeps the
    /// result, so [`Input::check`] and later calls read one document. A TOML
    /// input parses its text on each call.
    ///
    /// ```
    /// # #[cfg(feature = "input-yaml")]
    /// # {
    /// use ctl_core::input::{FieldPath, Input};
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
        self.places(std::slice::from_ref(path), At::Value)
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
    /// and a YAML document is built once and shared with [`Input::place`].
    ///
    /// ```
    /// # #[cfg(feature = "input-yaml")]
    /// # {
    /// use ctl_core::input::{FieldPath, Input};
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
        self.places(std::slice::from_ref(path), At::Key)
            .pop()
            .flatten()
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
        let found = match self.format {
            #[cfg(feature = "input-yaml")]
            Format::Yaml => yaml_text::places(self, &limits, paths, at),
            #[cfg(feature = "input-toml")]
            Format::Toml => toml_text::places(&self.text, paths, at),
        };
        found
            .into_iter()
            .map(|at| at.map(|(line, column)| Place::new(line + self.lines_before, column)))
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

/// The file at `path`, named by its path, or the I/O failure.
fn read_text(path: &Path) -> Result<(String, String), InputError> {
    let name = path.display().to_string();
    match std::fs::read_to_string(path) {
        Ok(text) => Ok((name, text)),
        Err(error) => Err(InputError::unread(name, &error)),
    }
}

impl InputError {
    /// A refusal of the whole input, not placed on a line.
    fn whole(name: String, message: String) -> Self {
        Self::of(
            name,
            vec![Problem {
                line: None,
                column: None,
                field: String::new(),
                message,
            }],
        )
    }

    /// The input `name` could not be read: the failure's text is the one
    /// problem, and its `io::ErrorKind` is kept.
    fn unread(name: String, error: &io::Error) -> Self {
        Self {
            io_failure: Some(error.kind()),
            ..Self::whole(name, error.to_string())
        }
    }

    /// The problems found in the input `name`, with no I/O failure.
    const fn of(name: String, problems: Vec<Problem>) -> Self {
        Self {
            name,
            problems,
            io_failure: None,
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

impl std::error::Error for InputError {}

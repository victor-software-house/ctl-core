//! Declared input: read a file, parse its shape with `serde-saphyr` (YAML) or
//! the `toml` crate (TOML, with `input-toml`), validate it once with garde,
//! and report every problem on its file and line.
//!
//! This is the one boundary where a CLI's config enters. Downstream code keeps
//! the validated type and never checks it again.
//!
//! A text's [`Format`] is chosen, never guessed: [`Input::open`] reads it from
//! the file's extension, and [`Input::new`] and [`Input::read`] are YAML.

use std::borrow::Cow;
use std::fmt;
use std::path::Path;

use garde::Validate;
use serde::de::DeserializeOwned;
use serde_saphyr::{
    Budget, DefaultMessageFormatter, DuplicateKeyPolicy, Error, Localizer, Location, MergeKeyPolicy,
};

mod limits;
#[cfg(feature = "input-toml")]
mod toml_text;

use limits::{Exceeded, Raise};
pub use limits::{Limits, Loosened, REPO_FILE, set_default_limits, use_repo_limits};

/// How a text is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// YAML, read by serde-saphyr and placed by yamled.
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
    /// An [`InputError`] with no line, naming the extension and the accepted
    /// ones, for any other extension or none; and for `.toml` when the
    /// `input-toml` feature is off.
    pub fn of(path: &Path) -> Result<Self, InputError> {
        let name = path.display().to_string();
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
        match extension.as_deref() {
            Some("yaml" | "yml") => Ok(Self::Yaml),
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

/// What a refusal tells a person to name the file.
#[cfg(feature = "input-toml")]
const ACCEPTED: &str = "name the file *.toml, *.yaml, or *.yml";
#[cfg(not(feature = "input-toml"))]
const ACCEPTED: &str = "name the file *.yaml or *.yml";

/// A named text, ready to parse and validate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    name: String,
    text: String,
    lines_before: usize,
    limits: Limits,
    format: Format,
}

/// Why an input was refused: every problem found, each placed on a line when
/// the text allows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputError {
    /// The input's name, usually its path.
    pub name: String,
    /// What is wrong, in the order found.
    pub problems: Vec<Problem>,
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
    /// A YAML input from text already in memory, whatever its name says. Set
    /// another format with [`Input::format`].
    #[must_use]
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
            lines_before: 0,
            limits: Limits::NONE,
            format: Format::Yaml,
        }
    }

    /// Read a YAML file, named by its path, whatever its extension.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the I/O failure.
    pub fn read(path: &Path) -> Result<Self, InputError> {
        let name = path.display().to_string();
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Self::new(name, text)),
            Err(error) => Err(InputError::whole(name, error.to_string())),
        }
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
        Ok(Self::read(path)?.format(format))
    }

    /// Set how the text is written, over what [`Input::new`] assumed.
    #[must_use]
    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// The input's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The input's text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Split YAML frontmatter from a Markdown body: the text between a first
    /// line of `---` and the next line of `---`. The frontmatter is YAML
    /// whatever this input's format. Problems in the frontmatter keep the
    /// line numbers of the whole file. `None` when the text does not start
    /// with a `---` line, or never closes it.
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
                };
                return Some((matter, &rest[offset + line.len()..]));
            }
            offset += line.len();
        }
        None
    }

    /// Override limits for this input alone, above every other tier: see
    /// [`Limits`] for the order.
    #[must_use]
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Parse the text's shape. In YAML only `true` and `false` are booleans,
    /// and a duplicate key or a merge key (`<<`) is refused; in TOML a
    /// duplicate key is refused by the format. The text must fit the limits in
    /// force, field by field from this input, the repository's
    /// [`REPO_FILE`], the process default, and serde-saphyr's own defaults.
    /// TOML reads only `max_reader_input_bytes`, `max_depth`, and `max_nodes`
    /// from them: the other fields name YAML syntax.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the parser's problem and its line. A limit
    /// problem names the limit, its value, and where to raise it.
    pub fn parse<T: DeserializeOwned>(&self) -> Result<T, InputError> {
        self.parse_within(
            &limits::effective(&self.limits).budget(),
            Raise::InRepository,
        )
    }

    /// [`Self::parse`] under exactly `budget`, whatever the tiers say.
    fn parse_within<T: DeserializeOwned>(
        &self,
        budget: &Budget,
        raise: Raise,
    ) -> Result<T, InputError> {
        if let Some(limit) = budget.max_reader_input_bytes
            && self.text.len() > limit
        {
            return Err(InputError::whole(
                self.name.clone(),
                format!(
                    "is {} bytes, which {}",
                    self.text.len(),
                    Exceeded::input_bytes(limit, raise)
                ),
            ));
        }
        match self.format {
            Format::Yaml => self.parse_yaml(budget, raise),
            #[cfg(feature = "input-toml")]
            Format::Toml => toml_text::parse(self, budget, raise),
        }
    }

    /// [`Self::parse_within`] for YAML, once the text's size is known to fit.
    fn parse_yaml<T: DeserializeOwned>(
        &self,
        budget: &Budget,
        raise: Raise,
    ) -> Result<T, InputError> {
        let options = serde_saphyr::options! {
            budget: Some(budget.clone()),
            strict_booleans: true,
            duplicate_keys: DuplicateKeyPolicy::Error,
            merge_keys: MergeKeyPolicy::Error,
            with_snippet: false,
        };
        serde_saphyr::from_str_with_options(&self.text, options).map_err(|error| {
            let location = error.location();
            InputError {
                name: self.name.clone(),
                problems: vec![Problem {
                    line: location
                        .and_then(|at| usize::try_from(at.line()).ok())
                        .map(|line| line + self.lines_before),
                    column: location.and_then(|at| usize::try_from(at.column()).ok()),
                    field: String::new(),
                    message: self.message(error.without_snippet(), budget, raise),
                }],
            }
        })
    }

    /// serde-saphyr's wording for `error`. An alias error also names where its
    /// value is defined, counted from the file like [`Problem::line`], and a
    /// limit error names the limit in `budget` and where to raise it.
    fn message(&self, error: &Error, budget: &Budget, raise: Raise) -> String {
        if let Error::Budget { breach, .. } = error
            && let Some(exceeded) = Exceeded::breach(breach, budget, raise)
        {
            return exceeded.to_string();
        }
        let Error::AliasError { msg, locations } = error else {
            return error.render_with_formatter(&DefaultMessageFormatter.with_localizer(&Unplaced));
        };
        let defined = locations.defined_location;
        let (text, innermost) = baked_positions(msg);
        let (line, column) = innermost.unwrap_or((
            usize::try_from(defined.line()).unwrap_or_default(),
            usize::try_from(defined.column()).unwrap_or_default(),
        ));
        let line = line + self.lines_before;
        format!("{text} (defined at line {line}, column {column})")
    }

    /// Validate a parsed value against `context`, and place each of garde's
    /// problems on the line of the field it names, or of its nearest written
    /// ancestor.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with every problem garde reports.
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
            .map(|(path, error)| (path.to_string(), error.message().to_owned()))
            .collect();
        let fields: Vec<_> = reported.iter().map(|(field, _)| field.as_str()).collect();
        let places = self.places(&fields);
        let problems = reported
            .into_iter()
            .zip(places)
            .map(|((field, message), place)| Problem {
                line: place.map(|(line, _)| line + self.lines_before),
                column: place.map(|(_, column)| column),
                field,
                message,
            })
            .collect();
        Err(InputError {
            name: self.name.clone(),
            problems,
        })
    }

    /// The 1-based line and column in the text of each garde field, or of its
    /// nearest written ancestor, counted in the text itself.
    fn places(&self, fields: &[&str]) -> Vec<Option<(usize, usize)>> {
        match self.format {
            Format::Yaml => {
                let document = yamled::Document::parse(self.text.as_str()).ok();
                fields
                    .iter()
                    .map(|field| {
                        document
                            .as_ref()
                            .and_then(|document| {
                                document.locate_nearest(&yaml_path(&field_steps(field)))
                            })
                            .map(|at| (at.line, at.column))
                    })
                    .collect()
            }
            #[cfg(feature = "input-toml")]
            Format::Toml => toml_text::places(&self.text, fields),
        }
    }

    /// Parse and validate with the default context.
    ///
    /// # Errors
    ///
    /// As [`Input::parse`] and [`Input::check`].
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

/// serde-saphyr's wording without the position it appends. That position
/// counts from the parsed text, which for frontmatter is not the file, and
/// [`Problem`] already carries the file's line and column.
struct Unplaced;

impl Localizer for Unplaced {
    fn attach_location<'a>(&self, base: Cow<'a, str>, _: Location) -> Cow<'a, str> {
        base
    }
}

/// The text of an [`Error::AliasError`] without the positions serde-saphyr
/// 1.3.0 wrote into it, and the innermost ` at line` one: where the failing
/// value is.
/// That text is rendered in English before any localizer runs, and an alias
/// reached through another alias nests one rendered error inside the next, so
/// each layer ends in ` (defined at line N, column M)`, ` at line N, column M`,
/// or both. Remove this once serde-saphyr keeps the inner error structured.
fn baked_positions(msg: &str) -> (&str, Option<(usize, usize)>) {
    let mut text = msg;
    let mut innermost = None;
    loop {
        if let Some((rest, _)) = trailing_position(text, " (defined at line ", ")") {
            text = rest;
        } else if let Some((rest, at)) = trailing_position(text, " at line ", "") {
            text = rest;
            innermost = Some(at);
        } else {
            break;
        }
    }
    (text, innermost)
}

/// `text` without a last `{lead}N, column M{close}`, with `N` and `M`.
fn trailing_position<'a>(
    text: &'a str,
    lead: &str,
    close: &str,
) -> Option<(&'a str, (usize, usize))> {
    let (rest, position) = text.strip_suffix(close)?.rsplit_once(lead)?;
    let (line, column) = position.split_once(", column ")?;
    Some((rest, (line.parse().ok()?, column.parse().ok()?)))
}

/// One step down a garde field path.
enum Step<'field> {
    /// A key of a mapping or table.
    Key(&'field str),
    /// An item of a sequence or array.
    Index(usize),
}

/// A garde field path (`packages[1].name`) as the steps down to it. A key
/// holding `.` or `[` cannot be told apart from nesting, so such a problem
/// lands on the nearest ancestor that resolves.
fn field_steps(field: &str) -> Vec<Step<'_>> {
    let mut steps = Vec::new();
    for part in field.split('.') {
        let mut pieces = part.split('[');
        if let Some(key) = pieces.next().filter(|key| !key.is_empty()) {
            steps.push(Step::Key(key));
        }
        for index in pieces {
            let index = index.trim_end_matches(']');
            steps.push(match index.parse::<usize>() {
                Ok(index) => Step::Index(index),
                Err(_) => Step::Key(index),
            });
        }
    }
    steps
}

/// `steps` as a yamled path.
fn yaml_path(steps: &[Step<'_>]) -> yamled::Path {
    steps
        .iter()
        .fold(yamled::Path::root(), |path, step| match step {
            Step::Key(key) => path.key(*key),
            Step::Index(index) => path.index(*index),
        })
}

impl InputError {
    fn whole(name: String, message: String) -> Self {
        Self {
            name,
            problems: vec![Problem {
                line: None,
                column: None,
                field: String::new(),
                message,
            }],
        }
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

#[cfg(test)]
mod tests {
    use super::baked_positions;

    #[test]
    fn every_written_position_is_taken_off_the_text() {
        assert_eq!(
            baked_positions("invalid u16 (defined at line 2, column 3)"),
            ("invalid u16", None)
        );
        assert_eq!(
            baked_positions(
                "invalid u16 at line 2, column 9 (defined at line 2, column 9) at line 3, column 7"
            ),
            ("invalid u16", Some((2, 9)))
        );
    }
}

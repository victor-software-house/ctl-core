//! Declared input: read a file, parse its shape with `serde-saphyr`, validate
//! it once with garde, and report every problem on its file and line.
//!
//! This is the one boundary where a CLI's config enters. Downstream code keeps
//! the validated type and never checks it again.

use std::borrow::Cow;
use std::fmt;
use std::path::Path;

use garde::Validate;
use serde::de::DeserializeOwned;
use serde_saphyr::{
    DefaultMessageFormatter, DuplicateKeyPolicy, Error, Localizer, Location, MergeKeyPolicy,
};

/// A named YAML text, ready to parse and validate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    name: String,
    text: String,
    lines_before: usize,
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
    /// An input from text already in memory.
    #[must_use]
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
            lines_before: 0,
        }
    }

    /// Read a file, named by its path.
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
    /// line of `---` and the next line of `---`. Problems in the frontmatter
    /// keep the line numbers of the whole file. `None` when the text does not
    /// start with a `---` line, or never closes it.
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
                };
                return Some((matter, &rest[offset + line.len()..]));
            }
            offset += line.len();
        }
        None
    }

    /// Parse the text's shape. Only `true` and `false` are booleans, and a
    /// duplicate key or a merge key (`<<`) is refused.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with the parser's problem and its line.
    pub fn parse<T: DeserializeOwned>(&self) -> Result<T, InputError> {
        let options = serde_saphyr::options! {
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
                    message: self.message(error.without_snippet()),
                }],
            }
        })
    }

    /// serde-saphyr's wording for `error`. An alias error also names where its
    /// value is defined, counted from the file like [`Problem::line`].
    fn message(&self, error: &Error) -> String {
        let Error::AliasError { msg, locations } = error else {
            return error.render_with_formatter(&DefaultMessageFormatter.with_localizer(&Unplaced));
        };
        let defined = locations.defined_location;
        let (text, line, column) = baked_position(msg).unwrap_or((
            msg.as_str(),
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
        let document = yamled::Document::parse(self.text.as_str()).ok();
        let problems = report
            .iter()
            .map(|(path, error)| {
                let field = path.to_string();
                let location = document
                    .as_ref()
                    .and_then(|document| document.locate_nearest(&yaml_path(&field)));
                Problem {
                    line: location.map(|at| at.line + self.lines_before),
                    column: location.map(|at| at.column),
                    field,
                    message: error.message().to_owned(),
                }
            })
            .collect();
        Err(InputError {
            name: self.name.clone(),
            problems,
        })
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
/// 1.3.0 wrote into it, and the innermost of them: where the failing value is.
/// That text is rendered in English before any localizer runs, and an alias
/// reached through another alias nests one rendered error inside the next, so
/// each layer ends in ` (defined at line N, column M)`, ` at line N, column M`,
/// or both. Remove this once serde-saphyr keeps the inner error structured.
fn baked_position(msg: &str) -> Option<(&str, usize, usize)> {
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
    innermost.map(|(line, column)| (text, line, column))
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

/// A garde field path (`packages[1].name`) as a yamled path. A key holding
/// `.` or `[` cannot be told apart from nesting, so such a problem lands on
/// the nearest ancestor that resolves.
fn yaml_path(field: &str) -> yamled::Path {
    let mut path = yamled::Path::root();
    for part in field.split('.') {
        let mut pieces = part.split('[');
        if let Some(key) = pieces.next().filter(|key| !key.is_empty()) {
            path = path.key(key);
        }
        for index in pieces {
            let index = index.trim_end_matches(']');
            path = match index.parse::<usize>() {
                Ok(index) => path.index(index),
                Err(_) => path.key(index),
            };
        }
    }
    path
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

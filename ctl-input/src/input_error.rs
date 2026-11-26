//! The error an input is refused with, and the problems it lists.

use std::error::Error;
use std::path::Path;
use std::{fmt, fs, io};

#[cfg(doc)]
use super::Input;

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

/// The file at `path`, named by its path, or the I/O failure.
pub(crate) fn read_text(path: &Path) -> Result<(String, String), InputError> {
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
    pub(crate) fn unread(name: String, error: &io::Error) -> Self {
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

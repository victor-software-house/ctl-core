//! The wording of every problem, in one place, and the process default that
//! replaces it.

use std::sync::{PoisonError, RwLock};

use super::format::{Fault, Raise};
#[cfg(feature = "yaml")]
use super::limits::REPO_FILE;

/// The words of every [`Fault`], so a CLI can write them in another language
/// or another voice.
///
/// A fault's variant and span stay the same whatever words it gets. The trait
/// words a fault and nothing else: the line, the column and the field come
/// from where the fault is.
///
/// ```
/// # #[cfg(any(feature = "yaml", feature = "toml"))]
/// # {
/// use ctl_input::format::Fault;
/// use ctl_input::{EnglishMessages, Messages};
///
/// let fault = Fault::MissingField { field: "port" };
/// assert_eq!(EnglishMessages.message(&fault), "missing field `port`");
/// # }
/// ```
pub trait Messages: Sync {
    /// The words for `fault`.
    fn message(&self, fault: &Fault) -> String;
}

/// The English wording, which names no parser option.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnglishMessages;

impl Messages for EnglishMessages {
    fn message(&self, fault: &Fault) -> String {
        match fault {
            Fault::Syntax { detail } | Fault::InvalidValue { detail } => detail.clone(),
            Fault::DuplicateKey { key } => format!("duplicate key `{key}`"),
            Fault::MergeKey => "merge key `<<` is not allowed".to_owned(),
            Fault::ExpectedKind { expected, found } => {
                format!("expected {expected}, found {found}")
            }
            Fault::UnknownField { field, expected } => {
                format!("unknown field `{field}`{}", choices(expected, "fields"))
            }
            Fault::UnknownVariant { variant, expected } => {
                format!(
                    "unknown variant `{variant}`{}",
                    choices(expected, "variants")
                )
            }
            Fault::MissingField { field } => format!("missing field `{field}`"),
            Fault::Limit { name, value, raise } => match raise {
                #[cfg(feature = "yaml")]
                Raise::InRepository => {
                    format!("exceeds {name} ({value}); raise limits.{name} in {REPO_FILE}")
                }
                #[cfg(feature = "yaml")]
                Raise::InProcess => format!(
                    "exceeds {name} ({value}); raise {name} in the tool's process default, which reads {REPO_FILE}"
                ),
                #[cfg(not(feature = "yaml"))]
                Raise::InTool => format!("exceeds {name} ({value}); the tool sets this limit"),
            },
        }
    }
}

/// `text` with each control character escaped as Rust's `escape_debug` does,
/// so a name or a value a text wrote cannot move a terminal that prints the
/// problem.
pub(crate) fn printable(text: &str) -> String {
    let mut shown = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') {
            shown.extend(character.escape_debug());
        } else {
            shown.push(character);
        }
    }
    shown
}

/// What follows an unknown name: the names that exist, or that there are none.
fn choices(expected: &[&str], plural: &str) -> String {
    if expected.is_empty() {
        return format!(", there are no {plural}");
    }
    let names: Vec<String> = expected.iter().map(|name| format!("`{name}`")).collect();
    format!(", expected one of {}", names.join(", "))
}

static PROCESS: RwLock<Option<&'static dyn Messages>> = RwLock::new(None);

/// Set the wording the process uses for a [`Refusal`](crate::format::Refusal)'s
/// text. Without a call, the process uses [`EnglishMessages`].
pub fn set_messages(messages: &'static dyn Messages) {
    *PROCESS.write().unwrap_or_else(PoisonError::into_inner) = Some(messages);
}

/// The wording the process uses: what [`set_messages`] installed, or English.
pub(crate) fn messages() -> &'static dyn Messages {
    PROCESS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .unwrap_or(&EnglishMessages)
}

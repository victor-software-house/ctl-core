//! The TOML half of [`Input::top_level`]. The `toml` crate's span-carrying
//! tree holds the root table's entries, in the order they were written.

use serde::de::IgnoredAny;
use toml::de::{DeTable, DeValue};

use super::entries::{Entry, Written};
use super::{Input, InputError};

/// [`Input::top_level`] for a TOML text, once [`Input::parse`] has read it
/// under the limits in force.
pub(super) fn entries(input: &Input) -> Result<Vec<Entry>, InputError> {
    input.parse::<IgnoredAny>()?;
    let root = DeTable::parse(&input.text)
        .map_err(|error| InputError::whole(input.name.clone(), error.message().to_owned()))?;
    let mut entries: Vec<_> = root.get_ref().iter().collect();
    entries.sort_by_key(|(key, _)| key.span().start);
    Ok(entries
        .into_iter()
        .map(|(key, value)| {
            let span = value.span();
            let (written, string) = match value.get_ref() {
                DeValue::String(text) => (
                    string_written(input.text.get(span.clone()).unwrap_or_default()),
                    Some(text.to_string()),
                ),
                DeValue::Integer(_)
                | DeValue::Float(_)
                | DeValue::Boolean(_)
                | DeValue::Datetime(_) => (Written::Plain, None),
                DeValue::Array(_) | DeValue::Table(_) => (Written::Collection, None),
            };
            Entry {
                key: key.get_ref().to_string(),
                value: span,
                written,
                string,
            }
        })
        .collect())
}

/// How the string written as `text` is quoted.
fn string_written(text: &str) -> Written {
    if text.starts_with("\"\"\"") {
        Written::MultilineBasic
    } else if text.starts_with("'''") {
        Written::MultilineLiteral
    } else if text.starts_with('\'') {
        Written::SingleQuoted
    } else {
        Written::DoubleQuoted
    }
}

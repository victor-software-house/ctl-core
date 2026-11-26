//! The names of the output formats, and the open interface that writes them.
//!
//! [`FormatName`] names one format. A name is a value, not a variant of a
//! closed enum, so a CLI can register a format of its own and ctl-core's
//! built-in formats are ordinary members of the set. `pretty` is for people,
//! `json` is one JSON document, `tsv` is one record per line for pipes, `csv`
//! is comma-separated values, and `markdown` is Markdown text.
//!
//! `FormatName` needs no feature. With `cli`, `--format` parses into it and
//! rejects a name no format answers to. With `view`, the `Format` trait,
//! `Output`, `Sink`, and `Formats` write a result in the format a name selects.
//!
//! ```
//! use ctl_core::format::FormatName;
//!
//! assert_eq!(FormatName::JSON.as_str(), "json");
//! assert_eq!(FormatName::new("yaml").to_string(), "yaml");
//! assert_ne!(FormatName::PRETTY, FormatName::JSON);
//! ```
#![cfg_attr(
    not(feature = "view"),
    doc = "",
    doc = "[`Format`]: crate#cargo-features",
    doc = "[`Output`]: crate#cargo-features",
    doc = "[`Sink`]: crate#cargo-features",
    doc = "[`Formats`]: crate#cargo-features"
)]

#[cfg(feature = "cli")]
use std::ffi::OsStr;
use std::fmt;

#[cfg(feature = "cli")]
use clap::builder::{PossibleValue, TypedValueParser};
#[cfg(feature = "cli")]
use clap::error::ErrorKind;
#[cfg(feature = "cli")]
use clap::parser::ValueSource;
#[cfg(feature = "cli")]
use clap::{Arg, Command};
#[cfg(feature = "json")]
use serde::{Serialize, Serializer};

#[cfg(feature = "view")]
mod csv;
#[cfg(feature = "view")]
mod interface;
#[cfg(feature = "view")]
mod json;
#[cfg(feature = "view")]
mod markdown;
#[cfg(feature = "view")]
mod output;
#[cfg(feature = "view")]
mod pretty;
#[cfg(feature = "view")]
mod registry;
#[cfg(feature = "view")]
mod stream;
#[cfg(feature = "view")]
mod tabular;
#[cfg(feature = "view")]
mod tsv;

#[cfg(feature = "view")]
pub use csv::{CSV, Csv};
#[cfg(feature = "view")]
pub use erased_serde;
#[cfg(feature = "view")]
pub use interface::Format;
#[cfg(feature = "view")]
pub use json::{JSON, Json};
#[cfg(feature = "view")]
pub use markdown::{MARKDOWN, Markdown};
#[cfg(feature = "view")]
pub use output::{Output, Sink};
#[cfg(feature = "view")]
pub use pretty::{PRETTY, Pretty};
#[cfg(feature = "view")]
pub use registry::Formats;
#[cfg(feature = "view")]
pub(crate) use registry::unregistered;
#[cfg(all(
    feature = "app",
    any(feature = "input-yaml", feature = "input-json", feature = "input-toml")
))]
pub(crate) use registry::{declares, format_prefix};
#[cfg(feature = "view")]
pub use stream::RecordWriter;
#[cfg(feature = "view")]
pub use tsv::{TSV, Tsv};

/// The help line of `pretty`, in `--format`'s help and completion and in
/// [`Format::about`], so the two cannot differ.
#[cfg(any(feature = "cli", feature = "view"))]
pub(crate) const PRETTY_ABOUT: &str = "Human text, optionally coloured";

/// The help line of `json`; see [`PRETTY_ABOUT`].
#[cfg(any(feature = "cli", feature = "view"))]
pub(crate) const JSON_ABOUT: &str = "Machine JSON. Never contains ANSI";

/// The help line of `tsv`; see [`PRETTY_ABOUT`].
#[cfg(any(feature = "cli", feature = "view"))]
pub(crate) const TSV_ABOUT: &str = "Tab-separated lines, one record per line";

/// The help line of `csv`; see [`PRETTY_ABOUT`].
#[cfg(any(feature = "cli", feature = "view"))]
pub(crate) const CSV_ABOUT: &str = "Comma-separated values (RFC 4180)";

/// The help line of `markdown`; see [`PRETTY_ABOUT`].
#[cfg(any(feature = "cli", feature = "view"))]
pub(crate) const MARKDOWN_ABOUT: &str = "Markdown tables and text";

/// The name of an output format: the value `--format` takes and `View::format`
/// returns.
///
/// Two names are equal when their text is. The built-in names are associated
/// constants, so a comparison reads `name == FormatName::JSON`. A CLI that
/// registers a format of its own declares its name once, as a `const`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct FormatName(&'static str);

impl FormatName {
    /// `pretty`: text for people, coloured when the stream allows it.
    pub const PRETTY: Self = Self::new("pretty");
    /// `json`: one JSON document.
    pub const JSON: Self = Self::new("json");
    /// `tsv`: one record per line, for pipes.
    pub const TSV: Self = Self::new("tsv");
    /// `csv`: comma-separated records.
    pub const CSV: Self = Self::new("csv");
    /// `markdown`: Markdown text.
    pub const MARKDOWN: Self = Self::new("markdown");

    /// The name `name`, which is the text `--format` takes. Use lowercase
    /// ASCII, as the built-in names do.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// The name's text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for FormatName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[cfg(feature = "json")]
impl Serialize for FormatName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

/// The names `--format` accepts, each with its help line.
///
/// One list feeds the accepted values, the possible values Clap reports to
/// help, completion, the Usage spec, and `Surface`, so none of them can list
/// a name the parser rejects.
#[cfg(feature = "cli")]
#[derive(Clone, Debug)]
pub(crate) struct FormatParser {
    names: Vec<(FormatName, &'static str)>,
}

#[cfg(feature = "cli")]
impl FormatParser {
    /// A parser for `names`, in the order help lists them.
    pub(crate) fn new(names: Vec<(FormatName, &'static str)>) -> Self {
        Self { names }
    }

    /// The parser the derived flags carry: the five built-in names. A CLI's
    /// registry replaces it when [`App`](crate::App) augments the command.
    pub(crate) fn builtin() -> Self {
        Self::new(vec![
            (FormatName::PRETTY, PRETTY_ABOUT),
            (FormatName::JSON, JSON_ABOUT),
            (FormatName::TSV, TSV_ABOUT),
            (FormatName::CSV, CSV_ABOUT),
            (FormatName::MARKDOWN, MARKDOWN_ABOUT),
        ])
    }

    /// `value` as a registered name, or the error that names it, the
    /// variable it came from when there is one, and the accepted names.
    fn parse(&self, value: &OsStr, variable: Option<&str>) -> Result<FormatName, clap::Error> {
        let Some(text) = value.to_str() else {
            return Err(clap::Error::raw(
                ErrorKind::InvalidUtf8,
                "the format is not valid UTF-8\n",
            ));
        };
        self.names
            .iter()
            .find_map(|(name, _)| (name.as_str() == text).then_some(*name))
            .ok_or_else(|| {
                unknown_format(
                    text,
                    variable,
                    &mut self.names.iter().map(|(name, _)| *name),
                )
            })
    }
}

/// The usage error for a format name that is not registered: it names the
/// value, the variable it came from when there is one, and the accepted
/// names. Clap's own error for a rejected value prints only a generic line
/// when its error context is off, as it is here.
#[cfg(feature = "cli")]
pub(crate) fn unknown_format(
    text: &str,
    variable: Option<&str>,
    accepted: &mut dyn Iterator<Item = FormatName>,
) -> clap::Error {
    let accepted = listed(accepted);
    let source = variable.map_or_else(String::new, |name| format!(" from the variable `{name}`"));
    clap::Error::raw(
        ErrorKind::InvalidValue,
        format_args!("unknown format `{text}`{source}; use one of {accepted}\n"),
    )
}

/// The usage error for a stream default that is not registered: `--stream`
/// with no format named would use it. It names the stream default and the
/// accepted names.
#[cfg(feature = "app")]
pub(crate) fn unknown_stream_default(
    name: FormatName,
    accepted: &mut dyn Iterator<Item = FormatName>,
) -> clap::Error {
    let accepted = listed(accepted);
    clap::Error::raw(
        ErrorKind::InvalidValue,
        format_args!("the stream default `{name}` is not registered; use one of {accepted}\n"),
    )
}

/// The accepted names, separated by commas.
#[cfg(feature = "cli")]
fn listed(accepted: &mut dyn Iterator<Item = FormatName>) -> String {
    accepted
        .map(FormatName::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(feature = "cli")]
impl TypedValueParser for FormatParser {
    type Value = FormatName;

    fn parse_ref(
        &self,
        _command: &Command,
        _arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<FormatName, clap::Error> {
        self.parse(value, None)
    }

    /// A value from a variable names that variable in its error, so a caller
    /// who never typed `--format` can find what to unset.
    fn parse_ref_(
        &self,
        _command: &Command,
        arg: Option<&Arg>,
        value: &OsStr,
        source: ValueSource,
    ) -> Result<FormatName, clap::Error> {
        let variable = (source == ValueSource::EnvVariable)
            .then(|| arg.and_then(Arg::get_env))
            .flatten()
            .and_then(OsStr::to_str);
        self.parse(value, variable)
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(self.names.iter().map(|(name, about)| {
            PossibleValue::new(name.as_str()).help(*about)
        })))
    }
}

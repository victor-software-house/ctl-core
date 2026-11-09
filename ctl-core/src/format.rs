//! The output representation: `pretty` for people, or `json` for machines.
//!
//! [`OutputFormat`] parses from those two lowercase names, and `Display`
//! prints them back. With the `cli` feature it is a Clap value enum, so
//! `OutputArgs` parses `--format` into it.
//!
//! ```
//! use ctl_core::format::OutputFormat;
//!
//! assert_eq!("json".parse::<OutputFormat>(), Ok(OutputFormat::Json));
//! assert!(OutputFormat::Json.is_json());
//! assert_eq!(OutputFormat::default(), OutputFormat::Pretty);
//! ```

use std::fmt;
use std::str::FromStr;

/// Hosts `JsonSchema`. schemars expands `concat!`; this module is the allow.
mod data {
    #![expect(
        clippy::disallowed_macros,
        reason = "schemars' JsonSchema derive expands concat!"
    )]

    /// The output representation a view writes a result in.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    #[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "json", serde(rename_all = "lowercase"))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub enum OutputFormat {
        #[default]
        /// Human text, optionally coloured.
        Pretty,
        /// Machine JSON. Never contains ANSI.
        Json,
    }
}

pub use data::OutputFormat;

impl OutputFormat {
    #[must_use]
    /// Whether this is [`Self::Json`], so the output is for machines.
    pub fn is_json(self) -> bool {
        self == Self::Json
    }
}

impl FromStr for OutputFormat {
    type Err = ParseFormatError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            _ => Err(ParseFormatError),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pretty => "pretty",
            Self::Json => "json",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// The error [`OutputFormat`]'s `FromStr` returns for text that is not
/// `pretty` or `json`. `Display` prints `expected pretty or json`.
pub struct ParseFormatError;

impl fmt::Display for ParseFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected pretty or json")
    }
}

impl std::error::Error for ParseFormatError {}

#[cfg(test)]
mod tests {
    use super::OutputFormat;

    #[test]
    fn json_predicate() {
        assert!(OutputFormat::Json.is_json());
        assert!(!OutputFormat::Pretty.is_json());
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
    }
}

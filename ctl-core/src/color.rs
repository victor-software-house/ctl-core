//! The colour policy for pretty output: `auto`, `always`, or `never`.
//!
//! [`ColorMode`] parses from those three lowercase names, and `Display` prints
//! them back. With the `cli` feature it is a Clap value enum, so
//! `OutputArgs` parses `--color` into it. JSON
//! output ignores it and never contains ANSI.
//!
//! ```
//! use ctl_core::color::ColorMode;
//!
//! assert_eq!("always".parse::<ColorMode>(), Ok(ColorMode::Always));
//! assert_eq!(ColorMode::default(), ColorMode::Auto);
//! assert!("rainbow".parse::<ColorMode>().is_err());
//! ```

use std::fmt;
use std::str::FromStr;

/// Hosts `JsonSchema`. schemars expands `concat!`; this module is the allow.
mod data {
    #![expect(
        clippy::disallowed_macros,
        reason = "schemars' JsonSchema derive expands concat!"
    )]

    /// Pretty-output color policy. JSON never contains ANSI.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    #[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
    #[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "json", serde(rename_all = "lowercase"))]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    pub enum ColorMode {
        #[default]
        /// Honor the stream: color if it is a TTY.
        Auto,
        /// Always emit ANSI.
        Always,
        /// Never emit ANSI.
        Never,
    }
}

pub use data::ColorMode;

/// The same policy in ctl-term's terms, so a CLI that writes a widget through
/// [`crate::term::Console`] passes the colour mode its flags parsed.
#[cfg(any(feature = "view", feature = "help"))]
impl From<ColorMode> for ctl_term::ColorMode {
    fn from(mode: ColorMode) -> Self {
        match mode {
            ColorMode::Auto => Self::Auto,
            ColorMode::Always => Self::Always,
            ColorMode::Never => Self::Never,
        }
    }
}

impl FromStr for ColorMode {
    type Err = ParseColorError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err(ParseColorError),
        }
    }
}

impl fmt::Display for ColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// The error [`ColorMode`]'s `FromStr` returns for text that is not `auto`,
/// `always`, or `never`. `Display` prints `expected auto, always, or never`.
pub struct ParseColorError;

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected auto, always, or never")
    }
}

impl std::error::Error for ParseColorError {}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::ColorMode;

    #[test]
    fn parse_rejects_junk() {
        let doc = indoc! {"
            auto
            always
            never
        "};
        for line in doc.lines() {
            assert!(line.parse::<ColorMode>().is_ok(), "{line}");
        }
        assert!("rainbow".parse::<ColorMode>().is_err());
    }
}

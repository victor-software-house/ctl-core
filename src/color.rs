use std::fmt;
use std::str::FromStr;

/// Hosts `JsonSchema`. schemars expands `concat!`; this module is the allow.
mod data {
    #![allow(clippy::disallowed_macros)]

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

impl ColorMode {
    /// Map to `anstream`'s color choice.
    #[cfg(any(feature = "view", feature = "help"))]
    #[must_use]
    pub(crate) fn choice(self) -> anstream::ColorChoice {
        match self {
            Self::Auto => anstream::ColorChoice::Auto,
            Self::Always => anstream::ColorChoice::Always,
            Self::Never => anstream::ColorChoice::Never,
        }
    }

    /// Last `-c/--color` / `--no-color` in `args` wins. Help runs before clap
    /// parse.
    #[must_use]
    pub fn from_args<'a, I>(args: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut mode = Self::Auto;
        let mut args = args.into_iter().peekable();
        while let Some(arg) = args.next() {
            if arg == "--no-color" {
                mode = Self::Never;
                continue;
            }
            let value = match arg {
                "-c" | "--color" => match args.peek() {
                    Some(next) if !next.starts_with('-') => args.next(),
                    _ => None,
                },
                other => other.strip_prefix("--color="),
            };
            if let Some(parsed) = value.and_then(|value| value.parse().ok()) {
                mode = parsed;
            }
        }
        mode
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
/// Unknown color token.
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
    fn last_flag_wins() {
        assert_eq!(
            ColorMode::from_args(["bin", "--color", "always", "--no-color"]),
            ColorMode::Never
        );
        assert_eq!(
            ColorMode::from_args(["bin", "--no-color", "--color", "always"]),
            ColorMode::Always
        );
    }

    #[test]
    fn short_long_and_equals() {
        assert_eq!(
            ColorMode::from_args(["x", "--color=never"]),
            ColorMode::Never
        );
        assert_eq!(
            ColorMode::from_args(["x", "-c", "always"]),
            ColorMode::Always
        );
    }

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

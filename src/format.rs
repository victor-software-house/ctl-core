use std::fmt;
use std::str::FromStr;

/// Output representation. Models serialize first; views pick one of these.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[cfg_attr(feature = "json", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "json", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum OutputFormat {
    #[default]
    /// Human text, optionally colored.
    Pretty,
    /// Machine JSON. Never contains ANSI.
    Json,
}

impl OutputFormat {
    #[must_use]
    /// `true` when this view is JSON.
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
/// Unknown format token.
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

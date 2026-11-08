//! Colour policy and colour depth.

use anstyle::Color;

use crate::palette::{ansi256_to_ansi16, rgb_to_ansi16, rgb_to_ansi256};

/// Whether to write colour, as the caller asked.
///
/// [`Always`](Self::Always) and [`Never`](Self::Never) ignore the variables
/// that decide whether to write colour, `NO_COLOR`, `CLICOLOR`,
/// `CLICOLOR_FORCE`, `FORCE_COLOR`, and `TERM=dumb`, and ask no stream. The
/// depth and the width still follow their variables. [`Auto`](Self::Auto)
/// follows the order in the Colour section of the [crate docs](crate).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ColorMode {
    /// Decide from the environment and the stream.
    #[default]
    Auto,
    /// Write colour whatever the environment holds.
    Always,
    /// Write no colour whatever the environment holds.
    Never,
}

/// How many colours the terminal can show.
///
/// The variants are ordered from fewest colours to most, so
/// `ColorDepth::Ansi16 < ColorDepth::TrueColor`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ColorDepth {
    /// The 16 named colours.
    Ansi16,
    /// The 256-colour palette.
    Ansi256,
    /// 24-bit colour.
    TrueColor,
}

impl ColorDepth {
    /// `color` converted to the nearest colour this depth can show.
    ///
    /// A colour that already fits comes back unchanged. At [`Ansi256`]
    /// a 24-bit colour becomes the nearest palette colour. At [`Ansi16`] a
    /// 24-bit or palette colour becomes the nearest named colour, measured
    /// directly, so no colour is rounded twice.
    ///
    /// [`Ansi256`]: Self::Ansi256
    /// [`Ansi16`]: Self::Ansi16
    #[must_use]
    pub fn downgrade(self, color: Color) -> Color {
        match color {
            Color::Ansi(_) => color,
            Color::Ansi256(palette) => match self {
                Self::Ansi16 => Color::Ansi(ansi256_to_ansi16(palette)),
                Self::Ansi256 | Self::TrueColor => color,
            },
            Color::Rgb(rgb) => match self {
                Self::Ansi16 => Color::Ansi(rgb_to_ansi16(rgb)),
                Self::Ansi256 => Color::Ansi256(rgb_to_ansi256(rgb)),
                Self::TrueColor => color,
            },
        }
    }
}

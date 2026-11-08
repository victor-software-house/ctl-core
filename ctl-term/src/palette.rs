//! Nearest-colour conversion between the three colour depths.
//!
//! Every function is pure and total. "Nearest" is the smallest squared
//! distance in RGB, and a tie goes to the lower palette index.

use anstyle::{Ansi256Color, AnsiColor, RgbColor};

/// The RGB values of the 16 named colours, as xterm draws them. Terminals let
/// the user change these, so the table fixes one reference to measure against.
const ANSI16: [(AnsiColor, RgbColor); 16] = [
    (AnsiColor::Black, RgbColor(0, 0, 0)),
    (AnsiColor::Red, RgbColor(205, 0, 0)),
    (AnsiColor::Green, RgbColor(0, 205, 0)),
    (AnsiColor::Yellow, RgbColor(205, 205, 0)),
    (AnsiColor::Blue, RgbColor(0, 0, 238)),
    (AnsiColor::Magenta, RgbColor(205, 0, 205)),
    (AnsiColor::Cyan, RgbColor(0, 205, 205)),
    (AnsiColor::White, RgbColor(229, 229, 229)),
    (AnsiColor::BrightBlack, RgbColor(127, 127, 127)),
    (AnsiColor::BrightRed, RgbColor(255, 0, 0)),
    (AnsiColor::BrightGreen, RgbColor(0, 255, 0)),
    (AnsiColor::BrightYellow, RgbColor(255, 255, 0)),
    (AnsiColor::BrightBlue, RgbColor(92, 92, 255)),
    (AnsiColor::BrightMagenta, RgbColor(255, 0, 255)),
    (AnsiColor::BrightCyan, RgbColor(0, 255, 255)),
    (AnsiColor::BrightWhite, RgbColor(255, 255, 255)),
];

/// The six levels each component of the 6x6x6 colour cube can take.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// The first palette index of the colour cube.
const CUBE_START: u8 = 16;

/// The first palette index of the grey ramp.
const GREY_START: u8 = 232;

/// The number of steps in the grey ramp.
const GREY_STEPS: u8 = 24;

/// The squared distance between two colours.
fn distance(a: RgbColor, b: RgbColor) -> u32 {
    let square = |x: u8, y: u8| u32::from(x.abs_diff(y)).pow(2);
    square(a.0, b.0) + square(a.1, b.1) + square(a.2, b.2)
}

/// The cube level index nearest to one component.
fn cube_step(component: u8) -> u8 {
    (0..)
        .zip(CUBE_LEVELS)
        .fold((0, u32::MAX), |best, (step, level)| {
            let gap = u32::from(component.abs_diff(level));
            if gap < best.1 { (step, gap) } else { best }
        })
        .0
}

/// The component value of a cube level index.
fn cube_level(step: u8) -> u8 {
    CUBE_LEVELS
        .get(usize::from(step))
        .copied()
        .unwrap_or(u8::MAX)
}

/// The grey ramp step nearest to a brightness. The ramp runs from 8 to 238 in
/// steps of 10.
fn grey_step(brightness: u8) -> u8 {
    let rounded = (u16::from(brightness).saturating_sub(8) + 5) / 10;
    u8::try_from(rounded)
        .unwrap_or(GREY_STEPS)
        .min(GREY_STEPS - 1)
}

/// The RGB value of a palette index from the cube or the grey ramp.
fn extended_rgb(index: u8) -> RgbColor {
    if index >= GREY_START {
        let level = 8 + 10 * (index - GREY_START);
        return RgbColor(level, level, level);
    }
    let offset = index.saturating_sub(CUBE_START);
    RgbColor(
        cube_level(offset / 36),
        cube_level(offset / 6 % 6),
        cube_level(offset % 6),
    )
}

/// The nearest of the 16 named colours to `color`.
///
/// The reference values are xterm's, which is what most terminals draw by
/// default.
#[must_use]
pub fn rgb_to_ansi16(color: RgbColor) -> AnsiColor {
    ANSI16
        .iter()
        .fold((AnsiColor::Black, u32::MAX), |best, (named, rgb)| {
            let gap = distance(color, *rgb);
            if gap < best.1 { (*named, gap) } else { best }
        })
        .0
}

/// The nearest colour to `color` in the 256-colour palette.
///
/// The search covers the 6x6x6 colour cube (indexes 16 to 231) and the 24-step
/// grey ramp (232 to 255). It skips indexes 0 to 15, which a terminal lets the
/// user redefine, so the result looks the same on every terminal. The grey
/// wins only when it is strictly nearer than the cube colour.
#[must_use]
pub fn rgb_to_ansi256(color: RgbColor) -> Ansi256Color {
    let RgbColor(red, green, blue) = color;
    let (red_step, green_step, blue_step) = (cube_step(red), cube_step(green), cube_step(blue));
    let cube = RgbColor(
        cube_level(red_step),
        cube_level(green_step),
        cube_level(blue_step),
    );
    let cube_index = CUBE_START + 36 * red_step + 6 * green_step + blue_step;

    let mean = (u16::from(red) + u16::from(green) + u16::from(blue)) / 3;
    let step = grey_step(u8::try_from(mean).unwrap_or(u8::MAX));
    let grey_index = GREY_START + step;
    let level = 8 + 10 * step;
    let grey = RgbColor(level, level, level);

    if distance(color, grey) < distance(color, cube) {
        Ansi256Color(grey_index)
    } else {
        Ansi256Color(cube_index)
    }
}

/// The nearest of the 16 named colours to a colour of the 256-colour palette.
///
/// Indexes 0 to 15 are the named colours and map to themselves. The cube and
/// the grey ramp map to the named colour nearest to their RGB value.
#[must_use]
pub fn ansi256_to_ansi16(color: Ansi256Color) -> AnsiColor {
    color
        .into_ansi()
        .unwrap_or_else(|| rgb_to_ansi16(extended_rgb(color.index())))
}

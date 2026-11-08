//! The nearest-colour conversions, as pure functions.

use ctl_term::{ColorDepth, ansi256_to_ansi16, rgb_to_ansi16, rgb_to_ansi256};
use ctl_text::anstyle::{Ansi256Color, AnsiColor, Color, RgbColor};

#[test]
fn rgb_maps_to_the_nearest_palette_colour() {
    let cases = [
        ((0, 0, 0), 16),
        ((255, 255, 255), 231),
        ((255, 0, 0), 196),
        ((0, 255, 0), 46),
        ((0, 0, 255), 21),
        ((95, 135, 175), 67),
        ((128, 128, 128), 244),
        ((8, 8, 8), 232),
        ((238, 238, 238), 255),
        ((250, 250, 250), 231),
        ((4, 4, 4), 16),
    ];
    for ((red, green, blue), expected) in cases {
        let nearest = rgb_to_ansi256(RgbColor(red, green, blue));
        assert_eq!(nearest, Ansi256Color(expected), "({red}, {green}, {blue})");
    }
}

#[test]
fn rgb_never_maps_to_the_redefinable_colours() {
    for red in (0..=255).step_by(15) {
        for green in (0..=255).step_by(15) {
            for blue in (0..=255).step_by(15) {
                let nearest = rgb_to_ansi256(RgbColor(red, green, blue));
                assert!(nearest.index() >= 16, "({red}, {green}, {blue})");
            }
        }
    }
}

#[test]
fn rgb_maps_to_the_nearest_named_colour() {
    let cases = [
        ((0, 0, 0), AnsiColor::Black),
        ((255, 0, 0), AnsiColor::BrightRed),
        ((205, 0, 0), AnsiColor::Red),
        ((0, 255, 0), AnsiColor::BrightGreen),
        ((0, 0, 238), AnsiColor::Blue),
        ((255, 255, 0), AnsiColor::BrightYellow),
        ((255, 255, 255), AnsiColor::BrightWhite),
        ((127, 127, 127), AnsiColor::BrightBlack),
        ((220, 220, 220), AnsiColor::White),
    ];
    for ((red, green, blue), expected) in cases {
        assert_eq!(
            rgb_to_ansi16(RgbColor(red, green, blue)),
            expected,
            "({red}, {green}, {blue})"
        );
    }
}

#[test]
fn a_palette_colour_maps_to_the_nearest_named_colour() {
    let cases = [
        (16, AnsiColor::Black),
        (196, AnsiColor::BrightRed),
        (21, AnsiColor::Blue),
        (231, AnsiColor::BrightWhite),
        (244, AnsiColor::BrightBlack),
        (255, AnsiColor::White),
    ];
    for (index, expected) in cases {
        assert_eq!(ansi256_to_ansi16(Ansi256Color(index)), expected, "{index}");
    }
}

#[test]
fn the_first_sixteen_palette_colours_are_the_named_colours() {
    for index in 0..16 {
        let palette = Ansi256Color(index);
        let named = palette.into_ansi();
        assert_eq!(Some(ansi256_to_ansi16(palette)), named, "{index}");
    }
}

#[test]
fn a_colour_that_fits_the_depth_is_unchanged() {
    let named = Color::Ansi(AnsiColor::Cyan);
    let palette = Color::Ansi256(Ansi256Color(100));
    let rgb = Color::Rgb(RgbColor(1, 2, 3));
    for depth in [
        ColorDepth::Ansi16,
        ColorDepth::Ansi256,
        ColorDepth::TrueColor,
    ] {
        assert_eq!(depth.downgrade(named), named);
    }
    for depth in [ColorDepth::Ansi256, ColorDepth::TrueColor] {
        assert_eq!(depth.downgrade(palette), palette);
    }
    assert_eq!(ColorDepth::TrueColor.downgrade(rgb), rgb);
}

#[test]
fn a_colour_is_converted_to_the_nearest_the_depth_can_show() {
    let rgb = Color::Rgb(RgbColor(255, 0, 0));
    assert_eq!(
        ColorDepth::Ansi256.downgrade(rgb),
        Color::Ansi256(Ansi256Color(196))
    );
    assert_eq!(
        ColorDepth::Ansi16.downgrade(rgb),
        Color::Ansi(AnsiColor::BrightRed)
    );
    assert_eq!(
        ColorDepth::Ansi16.downgrade(Color::Ansi256(Ansi256Color(196))),
        Color::Ansi(AnsiColor::BrightRed)
    );
}

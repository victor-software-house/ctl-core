//! The encoder: what it writes at each depth, where it writes a sequence, and
//! when it ends a line with a reset.

use ctl_term::{ColorDepth, Encoder, RESET};
use ctl_text::anstyle::{Ansi256Color, AnsiColor, Effects, Reset, RgbColor, Style};

use crate::common::{fg, line, red};

fn encode(depth: Option<ColorDepth>, lines: &[ctl_text::Line]) -> String {
    Encoder::new(depth).encode(lines)
}

#[test]
fn the_reset_is_the_standard_sequence() {
    assert_eq!(RESET, Reset.to_string());
}

#[test]
fn colour_off_writes_the_plain_text_of_each_line() {
    let lines = [
        line(&[
            ("id ", red()),
            ("title", Style::new().effects(Effects::BOLD)),
        ]),
        line(&[("plain", Style::new())]),
        line(&[]),
    ];
    assert_eq!(encode(None, &lines), "id title\nplain\n\n");
}

#[test]
fn no_lines_encode_to_nothing() {
    assert_eq!(encode(Some(ColorDepth::TrueColor), &[]), "");
    assert_eq!(encode(None, &[]), "");
}

#[test]
fn a_plain_line_gets_no_sequence() {
    let lines = [line(&[("plain", Style::new())])];
    assert_eq!(encode(Some(ColorDepth::TrueColor), &lines), "plain\n");
}

#[test]
fn an_empty_line_is_a_newline() {
    assert_eq!(encode(Some(ColorDepth::TrueColor), &[line(&[])]), "\n");
}

#[test]
fn a_styled_span_is_opened_and_reset() {
    let lines = [line(&[("failed", red())])];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[31mfailed\u{1b}[0m\n"
    );
}

#[test]
fn a_sequence_is_written_only_where_the_style_changes() {
    let green = fg(AnsiColor::Green);
    let lines = [line(&[
        ("a", red()),
        ("b", red()),
        ("c", green),
        ("d", green),
    ])];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[31mab\u{1b}[0m\u{1b}[32mcd\u{1b}[0m\n"
    );
}

#[test]
fn a_style_ending_mid_line_is_reset_once() {
    let lines = [line(&[("a", red()), ("b", Style::new())])];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[31ma\u{1b}[0mb\n"
    );
}

#[test]
fn a_span_with_no_text_writes_nothing() {
    let green = fg(AnsiColor::Green);
    let lines = [
        line(&[("a", red()), ("", green), ("b", red())]),
        line(&[("", red())]),
    ];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[31mab\u{1b}[0m\n\n"
    );
}

#[test]
fn plain_text_before_a_style_needs_no_reset() {
    let lines = [line(&[("a", Style::new()), ("b", red())])];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "a\u{1b}[31mb\u{1b}[0m\n"
    );
}

#[test]
fn each_line_stands_alone() {
    let lines = [
        line(&[("first", red())]),
        line(&[("second", red())]),
        line(&[("third", Style::new())]),
    ];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[31mfirst\u{1b}[0m\n\u{1b}[31msecond\u{1b}[0m\nthird\n"
    );
}

#[test]
fn effects_are_written_with_the_style() {
    let bold_red = red().effects(Effects::BOLD);
    let lines = [line(&[("x", bold_red)])];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[1m\u{1b}[31mx\u{1b}[0m\n"
    );
}

#[test]
fn a_background_is_downgraded_with_the_foreground() {
    let style = Style::new().bg_color(Some(RgbColor(255, 0, 0).into()));
    let lines = [line(&[("x", style)])];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[101mx\u{1b}[0m\n"
    );
    assert_eq!(
        encode(Some(ColorDepth::Ansi256), &lines),
        "\u{1b}[48;5;196mx\u{1b}[0m\n"
    );
}

#[test]
fn colours_that_downgrade_to_one_colour_share_a_sequence() {
    let lines = [line(&[
        ("a", fg(RgbColor(250, 10, 10))),
        ("b", fg(RgbColor(255, 0, 0))),
    ])];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[91mab\u{1b}[0m\n"
    );
}

#[test]
fn an_underline_colour_is_dropped_at_16_colours_and_kept_above() {
    let style = Style::new()
        .effects(Effects::UNDERLINE)
        .underline_color(Some(RgbColor(255, 0, 0).into()));
    let lines = [line(&[("x", style)])];
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[4mx\u{1b}[0m\n"
    );
    assert_eq!(
        encode(Some(ColorDepth::Ansi256), &lines),
        "\u{1b}[4m\u{1b}[58;5;196mx\u{1b}[0m\n"
    );
    assert_eq!(
        encode(Some(ColorDepth::TrueColor), &lines),
        "\u{1b}[4m\u{1b}[58;2;255;0;0mx\u{1b}[0m\n"
    );
}

#[test]
fn a_span_with_only_an_underline_colour_is_plain_at_16_colours() {
    let style = Style::new().underline_color(Some(AnsiColor::Red.into()));
    let lines = [line(&[("x", style)])];
    assert_eq!(encode(Some(ColorDepth::Ansi16), &lines), "x\n");
}

/// One 24-bit colour at each depth.
#[test]
fn a_24_bit_colour_is_written_at_each_depth() {
    let lines = [line(&[("x", fg(RgbColor(255, 0, 0)))])];
    assert_eq!(
        encode(Some(ColorDepth::TrueColor), &lines),
        "\u{1b}[38;2;255;0;0mx\u{1b}[0m\n"
    );
    assert_eq!(
        encode(Some(ColorDepth::Ansi256), &lines),
        "\u{1b}[38;5;196mx\u{1b}[0m\n"
    );
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[91mx\u{1b}[0m\n"
    );
}

/// One 256-palette colour at each depth.
#[test]
fn a_palette_colour_is_written_at_each_depth() {
    let lines = [line(&[("x", fg(Ansi256Color(196)))])];
    for depth in [ColorDepth::TrueColor, ColorDepth::Ansi256] {
        assert_eq!(
            encode(Some(depth), &lines),
            "\u{1b}[38;5;196mx\u{1b}[0m\n",
            "{depth:?}"
        );
    }
    assert_eq!(
        encode(Some(ColorDepth::Ansi16), &lines),
        "\u{1b}[91mx\u{1b}[0m\n"
    );
}

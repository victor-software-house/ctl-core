//! Properties of the encoder, over random lines, styles, and depths.
#![expect(
    clippy::disallowed_macros,
    reason = "proptest! and prop_assert! expand to concat!, which the repository bans in its own code"
)]

use ctl_term::{ColorDepth, Encoder, RESET};
use ctl_text::anstyle::{Ansi256Color, AnsiColor, Color, Effects, RgbColor, Style};
use ctl_text::{Line, Span};
use proptest::prelude::*;

use crate::common::ESC;

const FRAGMENTS: &[&str] = &[
    "a",
    "word",
    " ",
    "日本",
    "e\u{301}",
    "🇧🇷",
    "[2J",
    "\u{1b}",
    "\u{1b}[31m",
    "\u{202e}",
    "\t",
    "\n",
];

fn any_depth() -> impl Strategy<Value = ColorDepth> {
    prop::sample::select(vec![
        ColorDepth::Ansi16,
        ColorDepth::Ansi256,
        ColorDepth::TrueColor,
    ])
}

fn any_color() -> impl Strategy<Value = Color> {
    prop_oneof![
        prop::sample::select(vec![
            AnsiColor::Black,
            AnsiColor::Red,
            AnsiColor::BrightGreen,
            AnsiColor::BrightWhite,
        ])
        .prop_map(Color::Ansi),
        any::<u8>().prop_map(|index| Color::Ansi256(Ansi256Color(index))),
        any::<(u8, u8, u8)>().prop_map(|(red, green, blue)| Color::Rgb(RgbColor(red, green, blue))),
    ]
}

fn any_style() -> impl Strategy<Value = Style> {
    (
        prop::option::of(any_color()),
        prop::option::of(any_color()),
        prop::option::of(any_color()),
        prop::collection::vec(
            prop::sample::select(vec![Effects::BOLD, Effects::ITALIC, Effects::UNDERLINE]),
            0..3,
        ),
    )
        .prop_map(|(foreground, background, underline, effects)| {
            Style::new()
                .fg_color(foreground)
                .bg_color(background)
                .underline_color(underline)
                .effects(
                    effects
                        .into_iter()
                        .fold(Effects::new(), |all, one| all | one),
                )
        })
}

fn any_line() -> impl Strategy<Value = Line> {
    prop::collection::vec(
        (prop::sample::select(FRAGMENTS.to_vec()), any_style()),
        0..8,
    )
    .prop_map(|spans| {
        let mut line = Line::new();
        for (text, style) in spans {
            line.push(Span::new(text, style));
        }
        line
    })
}

fn any_lines() -> impl Strategy<Value = Vec<Line>> {
    prop::collection::vec(any_line(), 0..6)
}

/// The text of `lines`, one per row, with no style.
fn plain_text(lines: &[Line]) -> String {
    let mut text = String::new();
    for line in lines {
        text.push_str(&line.to_string());
        text.push('\n');
    }
    text
}

/// Whether `text` holds an escape byte.
fn has_escape(text: &str) -> bool {
    text.contains(ESC)
}

/// Whether `color` is one a terminal of `depth` can show.
fn fits(depth: ColorDepth, color: Color) -> bool {
    match color {
        Color::Ansi(_) => true,
        Color::Ansi256(_) => depth >= ColorDepth::Ansi256,
        Color::Rgb(_) => depth >= ColorDepth::TrueColor,
    }
}

proptest! {
    #[test]
    fn colourless_output_has_no_escape_byte_and_is_the_plain_text(lines in any_lines()) {
        let encoded = Encoder::new(None).encode(&lines);
        prop_assert!(!has_escape(&encoded));
        let plain = plain_text(&lines);
        prop_assert_eq!(encoded, plain);
    }

    #[test]
    fn coloured_output_leaves_no_style_open_at_the_end_of_any_line(
        lines in any_lines(),
        depth in any_depth(),
    ) {
        let encoded = Encoder::new(Some(depth)).encode(&lines);
        let encoded_lines: Vec<&str> = encoded.lines().collect();
        prop_assert_eq!(encoded_lines.len(), lines.len());
        for (encoded_line, line) in encoded_lines.iter().zip(&lines) {
            let styled = line
                .spans()
                .iter()
                .any(|span| !span.text().is_empty() && !span.style().is_plain());
            if !styled {
                prop_assert!(!has_escape(encoded_line), "{:?}", encoded_line);
            }
            if let Some(start) = encoded_line.rfind(ESC) {
                prop_assert!(encoded_line[start..].starts_with(RESET), "{:?}", encoded_line);
            }
        }
    }

    #[test]
    fn stripping_the_escapes_leaves_the_plain_text(lines in any_lines(), depth in any_depth()) {
        let encoded = Encoder::new(Some(depth)).encode(&lines);
        let plain = plain_text(&lines);
        let mut stripped = String::new();
        let mut rest = encoded.as_str();
        while let Some(start) = rest.find('\u{1b}') {
            stripped.push_str(&rest[..start]);
            let after = &rest[start..];
            let end = after.find('m').map_or(after.len(), |end| end + 1);
            rest = &after[end..];
        }
        stripped.push_str(rest);
        prop_assert_eq!(stripped, plain);
    }

    #[test]
    fn a_downgraded_colour_fits_its_depth_and_stays_put(color in any_color(), depth in any_depth()) {
        let fitted = depth.downgrade(color);
        prop_assert_eq!(depth.downgrade(fitted), fitted);
        prop_assert!(fits(depth, fitted), "{:?}", fitted);
    }
}

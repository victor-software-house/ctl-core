//! Properties of wrapping, folding, and truncating, over random text with
//! wide, combining, emoji, and control characters, and widths from 1 to 200.
#![expect(
    clippy::disallowed_macros,
    reason = "proptest! and prop_assert! expand to concat!, which the repository bans in its own code"
)]

use ctl_text::anstyle::Effects;
use ctl_text::unsafe_chars::is_unsafe;
use ctl_text::{Context, Line, Overflow, Renderable, Span, Style, Text, text_width};
use proptest::prelude::*;
use unicode_segmentation::UnicodeSegmentation;

const FRAGMENTS: &[&str] = &[
    "a",
    "word",
    "longerword",
    "x",
    " ",
    "   ",
    "\u{3000}",
    "\u{a0}",
    "日",
    "本語",
    "한",
    "e\u{301}",
    "\u{301}",
    "\u{200d}",
    "🇧🇷",
    "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}",
    "https://example.test/releases/tag/pkg@1.2.3",
    "[2J",
    "\n",
    "\u{1b}",
    "\u{202e}",
    "\u{2028}",
    "\u{7f}",
    "\r",
    "\u{c}",
    "\u{85}",
    "\u{200b}",
];

const STYLES: [Style; 3] = [
    Style::new(),
    Style::new().effects(Effects::BOLD),
    Style::new().effects(Effects::ITALIC),
];

/// Fragments with the style index each is pushed in.
type Parts = Vec<(&'static str, usize)>;

fn parts(fragments: Vec<&'static str>) -> impl Strategy<Value = Parts> {
    prop::collection::vec((prop::sample::select(fragments), 0..STYLES.len()), 0..40)
}

fn any_parts() -> impl Strategy<Value = Parts> {
    let mut fragments = FRAGMENTS.to_vec();
    fragments.push("\t");
    parts(fragments)
}

fn parts_without_tabs() -> impl Strategy<Value = Parts> {
    parts(FRAGMENTS.to_vec())
}

fn source(parts: &Parts) -> String {
    parts.iter().map(|(fragment, _)| *fragment).collect()
}

/// A text built from one push per fragment, so segments and styles vary.
fn text(parts: &Parts, overflow: Overflow) -> Text {
    let mut text = Text::new().overflow(overflow);
    for (fragment, style) in parts {
        text.push(fragment, STYLES[*style]);
    }
    text
}

fn any_width() -> impl Strategy<Value = u16> {
    1u16..=200
}

fn wrapping() -> impl Strategy<Value = Overflow> {
    prop::sample::select(vec![Overflow::KeepWords, Overflow::Fold])
}

fn any_overflow() -> impl Strategy<Value = Overflow> {
    prop::sample::select(vec![
        Overflow::KeepWords,
        Overflow::Fold,
        Overflow::Ellipsis,
    ])
}

/// Whitespace that may end a word: the Unicode `White_Space` set without the
/// no-break spaces, and the zero width space. Listed, not derived from
/// `char::is_whitespace`, so the oracle does not share the library's rule.
const BREAK_SPACES: &[char] = &[
    ' ', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{200b}', '\u{205f}', '\u{3000}',
];

const NO_BREAK_SPACES: &[char] = &['\u{a0}', '\u{2007}', '\u{202f}'];

fn render(parts: &Parts, width: u16, overflow: Overflow) -> Vec<Line> {
    text(parts, overflow).render(&Context::new(width))
}

fn is_space_cluster(cluster: &str) -> bool {
    cluster.chars().all(|c| BREAK_SPACES.contains(&c))
}

fn non_space_characters(text: &str) -> String {
    text.chars()
        .filter(|c| !BREAK_SPACES.contains(c) && !NO_BREAK_SPACES.contains(c))
        .collect()
}

/// What a span makes of one fragment: a line break becomes a space, and every
/// other unsafe character goes.
fn spanned(fragment: &str) -> String {
    fragment
        .chars()
        .filter_map(|c| match c {
            '\n' | '\r' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}' => Some(' '),
            c if is_unsafe(c) => None,
            c => Some(c),
        })
        .collect()
}

/// What a text makes of a source: a line break becomes a newline, and every
/// other unsafe character goes, tab and newline excepted.
fn texted(source: &str) -> String {
    source
        .chars()
        .filter_map(|c| match c {
            '\n' | '\t' => Some(c),
            '\r' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}' => Some('\n'),
            c if is_unsafe(c) => None,
            c => Some(c),
        })
        .collect()
}

fn non_space_clusters(text: &str) -> Vec<String> {
    text.split('\n')
        .flat_map(|line| line.graphemes(true))
        .filter(|cluster| !is_space_cluster(cluster))
        .map(str::to_owned)
        .collect()
}

fn joined_line(parts: &Parts) -> Line {
    parts
        .iter()
        .map(|(fragment, style)| Span::new(fragment, STYLES[*style]))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn spans_count_clusters_as_the_joined_text_does(parts in parts_without_tabs()) {
        let line = joined_line(&parts);
        let joined: String = parts.iter().map(|(fragment, _)| spanned(fragment)).collect();
        prop_assert_eq!(line.to_string(), joined.clone());
        prop_assert_eq!(line.width(), text_width(&joined));
        let per_span: Vec<&str> = line
            .spans()
            .iter()
            .flat_map(|span| span.text().graphemes(true))
            .collect();
        let whole: Vec<&str> = joined.graphemes(true).collect();
        prop_assert_eq!(per_span, whole);
    }

    #[test]
    fn neighbouring_spans_never_share_a_style(parts in parts_without_tabs()) {
        let line = joined_line(&parts);
        for pair in line.spans().windows(2) {
            prop_assert_ne!(pair[0].style(), pair[1].style());
        }
    }

    #[test]
    fn keep_words_lines_fit_or_hold_one_word(parts in any_parts(), width in any_width()) {
        for line in render(&parts, width, Overflow::KeepWords) {
            let text = line.to_string();
            prop_assert!(
                line.width() <= width || !text.graphemes(true).any(is_space_cluster),
                "{text:?} is {} cells at width {width}",
                line.width(),
            );
        }
    }

    #[test]
    fn fold_lines_fit_or_hold_one_cluster(parts in any_parts(), width in any_width()) {
        for line in render(&parts, width, Overflow::Fold) {
            let text = line.to_string();
            prop_assert!(
                line.width() <= width || text.graphemes(true).count() == 1,
                "{text:?} is {} cells at width {width}",
                line.width(),
            );
        }
    }

    #[test]
    fn ellipsis_lines_always_fit(parts in any_parts(), width in any_width()) {
        for line in render(&parts, width, Overflow::Ellipsis) {
            prop_assert!(line.width() <= width, "{line} is {} cells at width {width}", line.width());
        }
    }

    #[test]
    fn wrapping_keeps_every_non_space_character_once_and_in_order(
        parts in any_parts(),
        width in any_width(),
        overflow in wrapping(),
    ) {
        let rendered: String = render(&parts, width, overflow)
            .iter()
            .map(ToString::to_string)
            .collect();
        prop_assert_eq!(
            non_space_characters(&rendered),
            non_space_characters(&texted(&source(&parts))),
        );
    }

    #[test]
    fn wrapping_splits_no_cluster(
        parts in parts_without_tabs(),
        width in any_width(),
        overflow in wrapping(),
    ) {
        let rendered = render(&parts, width, overflow)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        prop_assert_eq!(
            non_space_clusters(&rendered),
            non_space_clusters(&texted(&source(&parts))),
        );
    }

    #[test]
    fn no_rendered_line_ends_in_a_space(
        parts in any_parts(),
        width in any_width(),
        overflow in any_overflow(),
    ) {
        for line in render(&parts, width, overflow) {
            let text = line.to_string();
            prop_assert!(
                !text.ends_with(|c: char| BREAK_SPACES.contains(&c)),
                "{text:?} ends in a space",
            );
        }
    }

    #[test]
    fn a_rendered_width_is_not_below_the_minimum_unless_the_target_is(
        parts in any_parts(),
        width in any_width(),
        overflow in any_overflow(),
    ) {
        let text = text(&parts, overflow);
        let context = Context::new(width);
        let minimum = text.measure(&context).minimum();
        let widest = text.render(&context).iter().map(Line::width).max().unwrap_or(0);
        prop_assert!(
            width < minimum || widest >= minimum,
            "widest {widest} below minimum {minimum} at width {width}",
        );
    }

    #[test]
    fn a_measurement_is_ordered_and_the_maximum_is_the_widest_unwrapped_line(
        parts in any_parts(),
        overflow in any_overflow(),
    ) {
        let text = text(&parts, overflow);
        let measurement = text.measure(&Context::new(80));
        let widest = text.render(&Context::unbounded()).iter().map(Line::width).max().unwrap_or(0);
        prop_assert!(measurement.minimum() <= measurement.maximum());
        prop_assert_eq!(measurement.maximum(), widest);
    }

    #[test]
    fn a_span_holds_no_unsafe_character_and_no_tab(parts in any_parts()) {
        let span = Span::new(source(&parts), Style::new());
        prop_assert!(
            span.text().chars().all(|c| !is_unsafe(c)),
            "{:?} holds an unsafe character",
            span.text(),
        );
    }

    #[test]
    fn truncation_keeps_a_prefix_of_the_text(parts in parts_without_tabs(), width in any_width()) {
        let line = joined_line(&parts);
        let cut = line.truncate(width).to_string();
        let original = line.to_string();
        match cut.strip_suffix('…') {
            Some(prefix) if cut != original => {
                prop_assert!(original.starts_with(prefix.trim_end_matches(' ')));
            }
            Some(_) | None => prop_assert_eq!(cut, original),
        }
    }

    #[test]
    fn truncation_of_a_wide_line_fills_the_width_exactly(
        parts in parts_without_tabs(),
        width in any_width(),
    ) {
        let line = joined_line(&parts);
        prop_assume!(line.width() > width);
        prop_assert_eq!(line.truncate(width).width(), width);
    }
}

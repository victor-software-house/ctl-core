//! Properties of wrapping, folding, and truncating, over random text with
//! wide, combining, emoji, and control characters, and widths from 1 to 200.
#![expect(
    clippy::disallowed_macros,
    reason = "proptest! and prop_assert! expand to concat!, which the repository bans in its own code"
)]

use ctl_text::verbatim::is_unsafe;
use ctl_text::{Context, Line, Overflow, Renderable, Span, Style, Text};
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
];

fn strategy(fragments: Vec<&'static str>) -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(fragments), 0..40).prop_map(|parts| parts.concat())
}

fn any_source() -> impl Strategy<Value = String> {
    let mut fragments = FRAGMENTS.to_vec();
    fragments.push("\t");
    strategy(fragments)
}

fn any_source_without_tabs() -> impl Strategy<Value = String> {
    strategy(FRAGMENTS.to_vec())
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

fn render(source: &str, width: u16, overflow: Overflow) -> Vec<Line> {
    Text::plain(source)
        .overflow(overflow)
        .render(&Context::new(width))
}

fn is_space_cluster(cluster: &str) -> bool {
    cluster
        .chars()
        .all(|c| c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}'))
}

fn non_space_characters(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

fn safe(source: &str) -> String {
    source
        .chars()
        .filter(|c| *c == '\n' || *c == '\t' || !is_unsafe(*c))
        .collect()
}

fn non_space_clusters(text: &str) -> Vec<String> {
    text.split('\n')
        .flat_map(|line| line.graphemes(true))
        .filter(|cluster| !is_space_cluster(cluster))
        .map(str::to_owned)
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn keep_words_lines_fit_or_hold_one_word(source in any_source(), width in any_width()) {
        for line in render(&source, width, Overflow::KeepWords) {
            let text = line.to_string();
            prop_assert!(
                line.width() <= width || !text.graphemes(true).any(is_space_cluster),
                "{text:?} is {} cells at width {width}",
                line.width(),
            );
        }
    }

    #[test]
    fn fold_lines_fit_or_hold_one_cluster(source in any_source(), width in any_width()) {
        for line in render(&source, width, Overflow::Fold) {
            let text = line.to_string();
            prop_assert!(
                line.width() <= width || text.graphemes(true).count() == 1,
                "{text:?} is {} cells at width {width}",
                line.width(),
            );
        }
    }

    #[test]
    fn ellipsis_lines_always_fit(source in any_source(), width in any_width()) {
        for line in render(&source, width, Overflow::Ellipsis) {
            prop_assert!(line.width() <= width, "{line} is {} cells at width {width}", line.width());
        }
    }

    #[test]
    fn wrapping_keeps_every_non_space_character_once_and_in_order(
        source in any_source(),
        width in any_width(),
        overflow in wrapping(),
    ) {
        let rendered: String = render(&source, width, overflow)
            .iter()
            .map(ToString::to_string)
            .collect();
        prop_assert_eq!(non_space_characters(&rendered), non_space_characters(&safe(&source)));
    }

    #[test]
    fn wrapping_splits_no_cluster(
        source in any_source_without_tabs(),
        width in any_width(),
        overflow in wrapping(),
    ) {
        let rendered = render(&source, width, overflow)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        prop_assert_eq!(non_space_clusters(&rendered), non_space_clusters(&safe(&source)));
    }

    #[test]
    fn no_rendered_line_ends_in_a_space(
        source in any_source(),
        width in any_width(),
        overflow in any_overflow(),
    ) {
        for line in render(&source, width, overflow) {
            let text = line.to_string();
            prop_assert!(
                !text.ends_with(' ') && !text.ends_with('\u{3000}'),
                "{text:?} ends in a space",
            );
        }
    }

    #[test]
    fn a_rendered_width_is_not_below_the_minimum_unless_the_target_is(
        source in any_source(),
        width in any_width(),
        overflow in any_overflow(),
    ) {
        let text = Text::plain(&source).overflow(overflow);
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
        source in any_source(),
        overflow in any_overflow(),
    ) {
        let text = Text::plain(&source).overflow(overflow);
        let context = Context::new(80);
        let measurement = text.measure(&context);
        let widest = text.render(&Context::new(u16::MAX)).iter().map(Line::width).max().unwrap_or(0);
        prop_assert!(measurement.minimum() <= measurement.maximum());
        prop_assert_eq!(measurement.maximum(), widest);
    }

    #[test]
    fn a_span_holds_no_unsafe_character_and_no_tab(source in any_source()) {
        let span = Span::new(&source, Style::new());
        prop_assert!(
            span.text().chars().all(|c| !is_unsafe(c)),
            "{:?} holds an unsafe character",
            span.text(),
        );
    }

    #[test]
    fn truncation_keeps_a_prefix_of_the_text(source in any_source_without_tabs(), width in any_width()) {
        let line = Line::from(Span::new(&source, Style::new()));
        let cut = line.truncate(width).to_string();
        let original = line.to_string();
        match cut.strip_suffix('…') {
            Some(prefix) if cut != original => prop_assert!(original.starts_with(prefix)),
            Some(_) | None => prop_assert_eq!(cut, original),
        }
    }
}

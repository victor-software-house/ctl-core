//! One test per scenario of the `styled-text` spec, in the spec's order.

use std::num::NonZeroU16;

use ctl_text::anstyle::AnsiColor;
use ctl_text::{
    Context, ELLIPSIS, Line, Measurement, Overflow, Renderable, Span, Style, Text, cluster_width,
    prefix_len, text_width,
};

const FAMILY: &str = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
const LAM_ALEF: &str = "\u{644}\u{627}";

fn render(text: &Text, width: u16) -> Vec<String> {
    text.render(&Context::new(width))
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn line_widths(text: &Text, width: u16) -> Vec<u16> {
    text.render(&Context::new(width))
        .iter()
        .map(Line::width)
        .collect()
}

#[test]
fn an_escape_sequence_in_a_title_loses_its_escape_byte() {
    let green = Style::new().fg_color(Some(AnsiColor::Green.into()));
    let text = Text::styled("fix \u{1b}[2Jthe build", green);
    let rendered = render(&text, 80);
    assert_eq!(rendered, ["fix [2Jthe build"]);
    assert!(rendered.iter().all(|line| !line.contains('\u{1b}')));
}

#[test]
fn a_span_removes_the_escape_byte_too() {
    let span = Span::new("fix \u{1b}[2Jthe build", Style::new());
    assert_eq!(span.text(), "fix [2Jthe build");
}

#[test]
fn a_right_to_left_override_is_removed() {
    assert_eq!(render(&Text::plain("abc\u{202e}def"), 80), ["abcdef"]);
}

#[test]
fn a_newline_starts_a_new_line() {
    assert_eq!(
        render(&Text::plain("first\nsecond"), 80),
        ["first", "second"]
    );
}

#[test]
fn a_blank_line_between_two_newlines_is_kept() {
    assert_eq!(render(&Text::plain("a\n\nb"), 80), ["a", "", "b"]);
}

#[test]
fn a_span_holds_one_line() {
    let span = Span::new("first\nsecond", Style::new());
    assert_eq!(span.text(), "first second");
}

#[test]
fn a_separator_in_a_span_becomes_a_space() {
    let span = Span::new("a\u{2028}b\u{2029}c\rd\u{c}e\u{85}f\ng", Style::new());
    assert_eq!(span.text(), "a b c d e f g");
}

#[test]
fn a_break_in_a_paragraph_starts_a_new_line() {
    let text = Text::plain("a\u{2028}b\u{2029}c\rd\u{c}e\u{85}f");
    assert_eq!(render(&text, 80), ["a", "b", "c", "d", "e", "f"]);
}

#[test]
fn a_carriage_return_and_a_line_feed_are_one_break() {
    assert_eq!(
        render(&Text::plain("first\r\nsecond"), 80),
        ["first", "second"]
    );
}

#[test]
fn an_empty_string_renders_to_no_lines() {
    assert_eq!(Text::plain(""), Text::new());
    assert_eq!(render(&Text::plain(""), 80), Vec::<String>::new());
    assert_eq!(
        Text::plain("").measure(&Context::new(80)),
        Measurement::default()
    );
}

#[test]
fn text_compares_by_content_however_it_was_pushed() {
    let bold = Style::new().bold();
    let mut pushed = Text::new();
    pushed.push("ab", Style::new());
    pushed.push("", bold);
    pushed.push("cd", Style::new());
    assert_eq!(pushed, Text::plain("abcd"));
    assert_ne!(pushed, Text::styled("abcd", bold));
}

#[test]
fn a_tab_expands_to_the_next_multiple_of_eight() {
    assert_eq!(
        render(&Text::plain("ab\tc"), 80),
        [format!("ab{}c", " ".repeat(6))]
    );
}

#[test]
fn a_tab_on_a_stop_moves_a_whole_stop() {
    let expected = format!("12345678{}x", " ".repeat(8));
    assert_eq!(render(&Text::plain("12345678\tx"), 80), [expected]);
}

#[test]
fn a_caller_sets_another_tab_width() {
    let tab = NonZeroU16::new(4).expect("4 is not zero");
    let context = Context::new(80).with_tab_width(tab);
    let lines = Text::plain("ab\tc").render(&context);
    assert_eq!(lines[0].to_string(), "ab  c");
}

#[test]
fn a_tab_counts_from_the_start_of_the_line_across_pushes() {
    let mut text = Text::new();
    text.push("ab", Style::new());
    text.push("\tc", Style::new().bold());
    assert_eq!(render(&text, 80), [format!("ab{}c", " ".repeat(6))]);
}

#[test]
fn a_tab_counts_the_width_of_wide_characters() {
    let expected = format!("日本{}x", " ".repeat(4));
    assert_eq!(render(&Text::plain("日本\tx"), 80), [expected]);
}

#[test]
fn a_tab_after_a_cluster_that_continues_from_an_earlier_push_lands_on_a_stop() {
    let mut text = Text::new();
    text.push("\u{2764}", Style::new());
    text.push("\u{fe0f}\tx", Style::new().bold());
    let rendered = render(&text, 80);
    assert_eq!(rendered, [format!("\u{2764}\u{fe0f}{}x", " ".repeat(6))]);
    assert_eq!(text_width(rendered[0].trim_end_matches('x')), 8);
}

#[test]
fn a_tab_after_a_zwj_sequence_built_across_pushes_lands_on_a_stop() {
    let mut text = Text::new();
    text.push("\u{1f468}", Style::new());
    text.push("\u{200d}\u{1f469}\tx", Style::new().italic());
    let rendered = render(&text, 80);
    assert_eq!(
        rendered,
        [format!("\u{1f468}\u{200d}\u{1f469}{}x", " ".repeat(6))]
    );
}

#[test]
fn a_family_emoji_is_two_cells() {
    assert_eq!(cluster_width(FAMILY), 2);
    assert_eq!(text_width(FAMILY), 2);
}

#[test]
fn wide_characters_are_two_cells_each() {
    assert_eq!(text_width("日本"), 4);
}

#[test]
fn letters_a_font_joins_count_one_cluster_each() {
    assert_eq!(cluster_width("\u{644}"), 1);
    assert_eq!(cluster_width("\u{627}"), 1);
    assert_eq!(text_width(LAM_ALEF), 2);
}

#[test]
fn east_asian_ambiguous_characters_are_one_cell() {
    assert_eq!(text_width("→±°"), 3);
}

#[test]
fn a_combining_mark_adds_no_width() {
    assert_eq!(text_width("e\u{301}"), 1);
}

#[test]
fn a_prefix_stops_before_a_cluster_that_does_not_fit() {
    assert_eq!(prefix_len("日本語", 3), "日".len());
    assert_eq!(prefix_len("日本語", 4), "日本".len());
    assert_eq!(prefix_len("abc", 9), 3);
    assert_eq!(prefix_len("abc", 0), 0);
}

#[test]
fn a_long_url_is_kept_whole() {
    let url = "https://example.test/releases/tag/pkg@1.2.3";
    let text = Text::plain(format!("see {url}"));
    assert_eq!(render(&text, 30), ["see", url]);
    let wider: Vec<u16> = line_widths(&text, 30)
        .into_iter()
        .filter(|width| *width > 30)
        .collect();
    assert_eq!(wider, [text_width(url)]);
}

#[test]
fn a_zero_width_space_is_a_place_to_break() {
    let text = Text::plain("ab\u{200b}cd");
    assert_eq!(render(&text, 2), ["ab", "cd"]);
    assert_eq!(render(&text, 4), ["ab\u{200b}cd"]);
    assert_eq!(text.measure(&Context::new(4)).minimum(), 2);
}

#[test]
fn words_break_at_spaces_only() {
    let text = Text::plain("one two three four");
    assert_eq!(render(&text, 9), ["one two", "three", "four"]);
}

#[test]
fn a_no_break_space_keeps_its_neighbours_together() {
    let text = Text::plain("a\u{a0}b c");
    assert_eq!(render(&text, 3), ["a\u{a0}b", "c"]);
}

#[test]
fn folding_wide_text_gives_one_cluster_per_line() {
    let text = Text::plain("日本語").overflow(Overflow::Fold);
    assert_eq!(render(&text, 3), ["日", "本", "語"]);
}

#[test]
fn folding_an_emoji_sequence_carries_it_whole() {
    let text = Text::plain(format!("ab{FAMILY}")).overflow(Overflow::Fold);
    assert_eq!(render(&text, 3), ["ab".to_owned(), FAMILY.to_owned()]);
}

#[test]
fn folding_leaves_a_word_that_fits_on_one_line() {
    let text = Text::plain("ab cdef gh").overflow(Overflow::Fold);
    assert_eq!(render(&text, 6), ["ab", "cdef", "gh"]);
}

#[test]
fn a_cluster_wider_than_the_width_keeps_a_line_of_its_own() {
    let text = Text::plain("a日b").overflow(Overflow::Fold);
    assert_eq!(render(&text, 1), ["a", "日", "b"]);
}

#[test]
fn truncation_ends_in_an_ellipsis() {
    let text = Text::plain("Let a ledger declare row separation").overflow(Overflow::Ellipsis);
    assert_eq!(render(&text, 12), ["Let a ledge…"]);
}

#[test]
fn truncation_leaves_text_that_fits_unchanged() {
    let text = Text::plain("Let a ledger").overflow(Overflow::Ellipsis);
    assert_eq!(render(&text, 12), ["Let a ledger"]);
}

#[test]
fn truncation_pads_a_wide_cluster_that_does_not_fit() {
    let line = Line::from(Span::new("日本語", Style::new()));
    let cut = line.truncate(4);
    assert_eq!(cut.to_string(), format!("日 {ELLIPSIS}"));
    assert_eq!(cut.width(), 4);
}

#[test]
fn truncation_to_one_cell_is_the_ellipsis_alone() {
    let line = Line::from(Span::new("abc", Style::new()));
    assert_eq!(line.truncate(1).to_string(), ELLIPSIS);
    assert!(line.truncate(0).is_empty());
}

#[test]
fn the_ellipsis_takes_the_style_of_the_text_it_replaces() {
    let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
    let line: Line = [Span::new("ab", Style::new()), Span::new("cdef", red)]
        .into_iter()
        .collect();
    let cut = line.truncate(4);
    assert_eq!(cut.to_string(), "abc…");
    let styles: Vec<Style> = cut.spans().iter().map(Span::style).collect();
    assert_eq!(styles, [Style::new(), red]);
    assert_eq!(cut.spans()[1].text(), "c…");
}

#[test]
fn a_borderless_line_with_a_short_value_ends_without_a_space() {
    let text = Text::plain("label   ");
    assert_eq!(render(&text, 80), ["label"]);
}

#[test]
fn a_wrapped_line_ends_without_a_space() {
    let text = Text::plain("one   two   three");
    let rendered = render(&text, 9);
    assert_eq!(rendered, ["one   two", "three"]);
}

#[test]
fn spaces_inside_a_line_are_kept() {
    assert_eq!(render(&Text::plain("a   b"), 80), ["a   b"]);
}

#[test]
fn indentation_stays_on_the_first_line() {
    assert_eq!(
        render(&Text::plain("  indented words here"), 12),
        ["  indented", "words here"]
    );
}

#[test]
fn a_truncated_line_ends_without_a_space_when_it_fits() {
    let text = Text::plain("abc   ").overflow(Overflow::Ellipsis);
    assert_eq!(render(&text, 20), ["abc"]);
}

#[test]
fn styles_survive_a_wrap() {
    let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
    let mut text = Text::new();
    text.push("plain ", Style::new());
    text.push("red words", red);
    let lines = text.render(&Context::new(8));
    let rendered: Vec<String> = lines.iter().map(ToString::to_string).collect();
    assert_eq!(rendered, ["plain", "red", "words"]);
    assert_eq!(lines[0].spans()[0].style(), Style::new());
    assert_eq!(lines[1].spans()[0].style(), red);
}

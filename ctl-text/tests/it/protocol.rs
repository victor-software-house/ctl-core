//! The widget protocol: measurements, the context, style overlay, and a
//! widget built on them.

use std::num::NonZeroU16;

use ctl_text::anstyle::{AnsiColor, Effects};
use ctl_text::{
    BoxChars, Context, Line, Measurement, Overflow, Renderable, Span, Style, TAB_WIDTH, Text,
    overlay,
};

fn lines(widget: &dyn Renderable, width: u16) -> Vec<String> {
    widget
        .render(&Context::new(width))
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn an_exact_measurement_has_equal_bounds() {
    let measurement = Measurement::exact(12);
    assert_eq!((measurement.minimum(), measurement.maximum()), (12, 12));
}

#[test]
fn the_default_measurement_is_empty() {
    assert_eq!(Measurement::default(), Measurement::exact(0));
}

#[test]
fn a_maximum_below_the_minimum_is_raised() {
    let measurement = Measurement::new(5, 2);
    assert_eq!((measurement.minimum(), measurement.maximum()), (5, 5));
}

#[test]
fn keep_words_measures_the_widest_word_and_the_widest_line() {
    let text = Text::plain("see https://example.test/releases/tag/pkg@1.2.3\nshort");
    let measurement = text.measure(&Context::new(30));
    assert_eq!(measurement.minimum(), 43);
    assert_eq!(measurement.maximum(), 47);
}

#[test]
fn fold_measures_the_widest_cluster() {
    let text = Text::plain("日本語 ab").overflow(Overflow::Fold);
    let measurement = text.measure(&Context::new(30));
    assert_eq!((measurement.minimum(), measurement.maximum()), (2, 9));
}

#[test]
fn ellipsis_measures_one_cell_for_text_and_none_for_nothing() {
    let cut = Text::plain("abc def").overflow(Overflow::Ellipsis);
    assert_eq!(cut.measure(&Context::new(30)).minimum(), 1);
    let blank = Text::plain("   ").overflow(Overflow::Ellipsis);
    let measurement = blank.measure(&Context::new(30));
    assert_eq!((measurement.minimum(), measurement.maximum()), (0, 0));
}

#[test]
fn empty_text_measures_and_renders_nothing() {
    let text = Text::new();
    assert_eq!(text.measure(&Context::new(30)), Measurement::exact(0));
    assert_eq!(text.render(&Context::new(30)), Vec::<Line>::new());
}

#[test]
fn a_measurement_counts_tabs_at_the_context_tab_width() {
    let text = Text::plain("a\tb");
    assert_eq!(text.measure(&Context::new(30)).maximum(), 9);
    let tab = NonZeroU16::new(2).expect("2 is not zero");
    let context = Context::new(30).with_tab_width(tab);
    assert_eq!(text.measure(&context).maximum(), 3);
}

#[test]
fn a_context_derives_a_child_context() {
    let tab = NonZeroU16::new(4).expect("4 is not zero");
    let parent = Context::new(80)
        .with_tab_width(tab)
        .with_box_chars(BoxChars::Ascii);
    let child = parent.with_width(20);
    assert_eq!(
        (child.width(), child.tab_width(), child.box_chars()),
        (20, tab, BoxChars::Ascii)
    );
    assert_eq!(parent.width(), 80);
}

#[test]
fn an_unbounded_context_stays_unbounded_through_every_with_method() {
    let tab = NonZeroU16::new(4).expect("4 is not zero");
    let context = Context::unbounded();
    assert_eq!((context.width(), context.is_unbounded()), (u16::MAX, true));
    assert!(!Context::new(u16::MAX).is_unbounded());
    assert!(context.with_width(20).is_unbounded());
    assert!(context.with_tab_width(tab).is_unbounded());
    assert!(context.with_box_chars(BoxChars::Ascii).is_unbounded());
    assert!(!Context::new(80).with_width(20).is_unbounded());
}

#[test]
fn a_context_defaults_to_tab_stops_of_eight_and_unicode_boxes() {
    let context = Context::new(80);
    assert_eq!(
        (context.tab_width(), context.box_chars()),
        (TAB_WIDTH, BoxChars::Unicode)
    );
    assert_eq!(TAB_WIDTH.get(), 8);
}

#[test]
fn a_container_holds_widgets_as_trait_objects() {
    let widgets: Vec<Box<dyn Renderable>> = vec![
        Box::new(Text::plain("one two")),
        Box::new(Text::plain("日本語").overflow(Overflow::Fold)),
    ];
    let rendered: Vec<Vec<String>> = widgets.iter().map(|widget| lines(widget, 3)).collect();
    assert_eq!(rendered, [vec!["one", "two"], vec!["日", "本", "語"]]);
}

fn measured_width(widget: impl Renderable) -> u16 {
    widget.measure(&Context::new(30)).maximum()
}

#[test]
fn a_reference_and_a_box_are_widgets_too() {
    let text = Text::plain("one two");
    assert_eq!(measured_width(&text), 7);
    assert_eq!(measured_width(Box::new(text.clone())), 7);
    let dynamic: Box<dyn Renderable> = Box::new(text.clone());
    assert_eq!(measured_width(&dynamic), 7);
    assert_eq!(measured_width(dynamic), 7);
    let borrowed: &dyn Renderable = &text;
    assert_eq!(measured_width(borrowed), 7);
}

#[test]
fn a_widget_draws_itself_from_a_measurement() {
    struct Bar(u16);

    impl Renderable for Bar {
        fn measure(&self, _: &Context) -> Measurement {
            Measurement::exact(self.0)
        }

        fn render(&self, context: &Context) -> Vec<Line> {
            let cells = self.0.min(context.width());
            vec![Line::from(Span::new(
                "=".repeat(cells.into()),
                Style::new(),
            ))]
        }
    }

    let bar = Bar(10);
    assert_eq!(bar.measure(&Context::new(4)), Measurement::exact(10));
    assert_eq!(lines(&bar, 4), ["===="]);
}

#[test]
fn overlay_prefers_the_top_colours_and_keeps_the_base_ones_it_leaves_unset() {
    let base = Style::new()
        .fg_color(Some(AnsiColor::Red.into()))
        .bg_color(Some(AnsiColor::White.into()));
    let top = Style::new().fg_color(Some(AnsiColor::Blue.into()));
    let merged = overlay(base, top);
    assert_eq!(merged.get_fg_color(), Some(AnsiColor::Blue.into()));
    assert_eq!(merged.get_bg_color(), Some(AnsiColor::White.into()));
}

#[test]
fn overlay_adds_the_effects_of_both() {
    let merged = overlay(Style::new().bold(), Style::new().underline());
    assert_eq!(merged.get_effects(), Effects::BOLD | Effects::UNDERLINE);
}

#[test]
fn overlaying_the_default_style_changes_nothing() {
    let base = Style::new()
        .fg_color(Some(AnsiColor::Green.into()))
        .italic();
    assert_eq!(overlay(base, Style::new()), base);
    assert_eq!(overlay(Style::new(), base), base);
}

#[test]
fn a_cluster_split_across_spans_counts_once() {
    let bold = Style::new().effects(Effects::BOLD);
    let family: Line = [
        Span::new("\u{1f468}", Style::new()),
        Span::new("\u{200d}\u{1f469}", bold),
    ]
    .into_iter()
    .collect();
    assert_eq!(family.width(), 2);
    assert_eq!(family.spans().len(), 1);

    let mut accent = Line::new();
    accent.push(Span::new("e", Style::new()));
    accent.push(Span::new("\u{301}x", bold));
    assert_eq!(accent.width(), 2);
    assert_eq!(accent.spans()[0].text(), "e\u{301}");
    assert_eq!(accent.spans()[1].text(), "x");
}

#[test]
fn an_empty_span_between_a_base_and_its_mark_adds_nothing() {
    let mut line = Line::new();
    line.push(Span::new("e", Style::new()));
    line.push(Span::new("", Style::new().effects(Effects::BOLD)));
    line.push(Span::new("\u{301}", Style::new().effects(Effects::ITALIC)));
    assert_eq!(line.width(), 1);
    assert_eq!(line.spans().len(), 1);
    assert_eq!(line.spans()[0].text(), "e\u{301}");
    assert_eq!(Line::from(Span::new("", Style::new())).spans().len(), 0);
}

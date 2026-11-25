//! One test per scenario of the `layout` spec, in the spec's order.

use std::iter;

use ctl_layout::{Column, Definitions, Fallback, Frame, Indent, Rule, Stack, Table};
use ctl_text::{Context, Measurement, Overflow, Renderable, Text};

use crate::common::{cells, flags, ledger, lines, paths, queue, releases, widest};

#[test]
fn a_record_measures_its_widest_line_as_key_two_spaces_and_value() {
    let measurement = ledger().measure(&Context::new(80));
    assert_eq!(measurement.maximum(), 18);
    assert_eq!(measurement.minimum(), 6 + 2 + 10);
}

#[test]
fn a_widget_draws_no_line_wider_than_its_width_but_a_kept_word() {
    let url = "https://example.test/releases/tag/pkg@1.2.3";
    let record = ctl_layout::Record::new().row(Text::plain("release"), Text::plain(url));
    let rendered = lines(&record, 30);
    let wide: Vec<&String> = rendered
        .iter()
        .filter(|line| ctl_text::text_width(line) > 30)
        .collect();
    assert_eq!(wide.len(), 1, "{rendered:?}");
    assert!(wide[0].ends_with(url), "{wide:?}");
}

#[test]
fn a_table_that_fits_takes_its_natural_widths() {
    let table = queue(Frame::HeaderRule, ctl_layout::RowSeparation::None);
    let rendered = lines(&table, 80);
    assert_eq!(widest(&rendered), 50, "{rendered:?}");
    assert_eq!(table.measure(&Context::new(80)).maximum(), 50);
}

#[test]
fn a_table_shrinks_its_widest_wrappable_column_first() {
    let title = format!("{} abcdef", ["abcdefg"; 8].join(" "));
    assert_eq!(ctl_text::text_width(&title), 70);
    let table = Table::new([Column::new("id"), Column::new("title")])
        .row([Text::plain("QCTL-014"), Text::plain(title)]);
    let rendered = lines(&table, 40);
    assert!(widest(&rendered) <= 40, "{rendered:?}");
    assert!(rendered[1].starts_with("│ id       ┆ "), "{rendered:?}");
    assert!(rendered[3].starts_with("│ QCTL-014 ┆ "), "{rendered:?}");
    assert!(rendered.len() > 6, "the title wraps: {rendered:?}");
}

#[test]
fn a_wrapped_column_narrows_to_its_widest_line() {
    let title = format!("{} abcdef", ["abcdefg"; 8].join(" "));
    let table = Table::new([Column::new("id"), Column::new("title")])
        .row([Text::plain("QCTL-014"), Text::plain(title)]);
    let rendered = lines(&table, 40);
    assert_eq!(widest(&rendered), 38, "{rendered:?}");
    assert!(
        rendered[3].ends_with("┆ abcdefg abcdefg abcdefg │"),
        "{rendered:?}"
    );
}

#[test]
fn ellipsis_columns_give_way_before_a_word_breaks() {
    let rendered = lines(&paths().frame(Frame::Plain), 40);
    assert_eq!(
        rendered,
        [
            format!("{:<30}  title", "path"),
            "crates/ctl-core/src/renders.rs  Let a l…".to_owned(),
            "crates/ctl-text/src/wrap.rs     ok".to_owned(),
        ]
    );
}

#[test]
fn a_grid_folds_a_path_before_an_ellipsis_column_goes_below_its_floor() {
    let rendered = lines(&paths(), 40);
    assert_eq!(widest(&rendered), 40, "{rendered:?}");
    assert!(
        !rendered
            .iter()
            .any(|line| line.contains("crates/ctl-core/src/renders.rs")),
        "the path folds: {rendered:?}"
    );
    assert!(
        rendered.iter().any(|line| line.contains("Let a l…")),
        "the title keeps its 8 cells: {rendered:?}"
    );
}

#[test]
fn a_framed_table_folds_instead_of_overflowing() {
    let rendered = lines(&releases(), 40);
    assert!(widest(&rendered) <= 40, "{rendered:?}");
    let url = format!("https://example.test/{}", "a".repeat(39));
    assert!(
        !rendered.iter().any(|line| line.contains(&url)),
        "{rendered:?}"
    );
}

#[test]
fn an_unframed_table_keeps_a_word_whole() {
    let table = releases().frame(Frame::Plain);
    let rendered = lines(&table, 40);
    let url = format!("https://example.test/{}", "a".repeat(39));
    assert!(
        rendered.iter().any(|line| line.contains(&url)),
        "{rendered:?}"
    );
}

#[test]
fn a_numeric_column_aligns_right() {
    let table = Table::new([Column::new("name"), Column::new("age").numeric()])
        .frame(Frame::Plain)
        .row(cells(["a", "3"]))
        .row(cells(["b", "120"]));
    let rendered = lines(&table, 80);
    assert_eq!(rendered, ["name  age", "a       3", "b     120"]);
}

#[test]
fn definitions_put_labels_over_an_indented_description() {
    let definitions = Definitions::new().entry(["-f", "--format"], ["Output representation"]);
    assert_eq!(
        lines(&definitions, 40),
        ["  -f --format", "    Output representation"]
    );
}

#[test]
fn a_breakpoint_draws_its_narrow_widget_below_the_threshold() {
    assert_eq!(lines(&flags(), 63)[0], "  -f --format");
}

#[test]
fn a_breakpoint_draws_its_wide_widget_at_the_threshold() {
    assert_eq!(
        lines(&flags(), 64)[0],
        "-f  --format  Output representation"
    );
}

#[test]
fn a_breakpoint_measures_the_widget_that_applies() {
    let narrow = flags().measure(&Context::new(63));
    let wide = flags().measure(&Context::new(64));
    assert_eq!(narrow.maximum(), 4 + 24);
    assert_eq!(wide.maximum(), 2 + 2 + 8 + 2 + 24);
}

#[test]
fn definition_labels_wrap_and_keep_their_indent() {
    let definitions = Definitions::new().entry(["one two three four five"], ["description"]);
    assert_eq!(
        lines(&definitions, 14),
        ["  one two", "  three four", "  five", "    description"]
    );
}

#[test]
fn a_rule_fills_the_width_and_a_title_sits_at_its_left() {
    assert_eq!(lines(&Rule::new(), 5), ["─────"]);
    assert_eq!(
        lines(&Rule::titled(Text::plain("Title")), 20),
        ["── Title ───────────"]
    );
}

#[test]
fn a_rule_title_that_does_not_fit_ends_in_an_ellipsis() {
    assert_eq!(
        lines(&Rule::titled(Text::plain("A very long title")), 12),
        ["── A very …"]
    );
}

#[test]
fn a_stack_puts_widgets_one_above_the_other_with_the_spacing_between() {
    let stack = Stack::new()
        .spacing(1)
        .push(Rule::new())
        .push(Text::plain("text").overflow(Overflow::Fold))
        .push(Text::new())
        .push(Text::plain("\n"));
    assert_eq!(lines(&stack, 6), ["──────", "", "text"]);
}

#[test]
fn a_stack_and_an_indent_can_borrow_their_children() {
    let rule = Rule::new();
    let stack = Stack::new().push(&rule);
    let indent = Indent::new(1, &rule);
    assert_eq!(lines(&stack, 5), ["─────"]);
    assert_eq!(lines(&indent, 5), [" ────"]);
    assert_eq!(lines(&rule, 5), ["─────"]);
}

#[test]
fn an_empty_table_draws_nothing_and_measures_zero_in_every_frame() {
    for frame in [Frame::Grid, Frame::HeaderRule, Frame::Plain] {
        let table = Table::new([Column::untitled(), Column::untitled()]).frame(frame);
        assert!(lines(&table, 80).is_empty(), "{frame:?}");
        assert_eq!(
            table.measure(&Context::new(80)),
            Measurement::exact(0),
            "{frame:?}"
        );
    }
}

#[test]
fn a_row_of_empty_cells_still_draws_one_blank_line() {
    let table = Table::new([Column::untitled(), Column::untitled()])
        .frame(Frame::Plain)
        .row(cells(["a", "b"]))
        .row(cells(["", ""]))
        .row(cells(["c", "d"]));
    assert_eq!(lines(&table, 80), ["a  b", "", "c  d"]);
}

#[test]
fn a_row_takes_str_string_and_text_cells() {
    let table = Table::new([Column::new("a"), Column::new(String::from("b"))])
        .frame(Frame::Plain)
        .row(["1", "2"])
        .row([String::from("3"), String::from("4")])
        .row([Text::plain("5"), Text::plain("6")]);
    assert_eq!(lines(&table, 80), ["a  b", "1  2", "3  4", "5  6"]);
}

#[test]
fn an_indent_moves_its_child_right_and_narrows_it() {
    let indent = Indent::new(2, Text::plain("one two three"));
    assert_eq!(lines(&indent, 9), ["  one two", "  three"]);
    assert_eq!(indent.measure(&Context::new(9)).maximum(), 15);
}

/// Six columns of 30-cell values under 4-cell headers.
fn six_paths() -> Table {
    let value = "x".repeat(30);
    Table::new((1..=6).map(|index| Column::new(format!("col{index}"))))
        .row(iter::repeat_n(value.as_str(), 6))
}

/// The content width of each column of a grid, read from its top rule.
fn grid_columns(rendered: &[String]) -> Vec<usize> {
    rendered[0]
        .trim_matches(['┌', '┐'])
        .split('┬')
        .map(|segment| segment.chars().count() - 2)
        .collect()
}

#[test]
fn a_wide_list_keeps_every_column_at_its_floor_or_more() {
    let rendered = lines(&six_paths(), 80);
    assert!(widest(&rendered) <= 80, "{rendered:?}");
    let columns = grid_columns(&rendered);
    assert_eq!(columns.len(), 6, "{rendered:?}");
    assert!(columns.iter().all(|width| *width >= 8), "{columns:?}");
}

#[test]
fn six_wide_columns_do_not_fit_at_40_and_run_past_it_at_their_floors() {
    let table = six_paths();
    assert!(!table.fits(&Context::new(40)));
    let rendered = lines(&table, 40);
    assert_eq!(grid_columns(&rendered), [8; 6], "{rendered:?}");
    assert_eq!(widest(&rendered), 19 + 6 * 8, "{rendered:?}");
}

#[test]
fn a_fallback_stacks_the_table_that_does_not_fit() {
    let table = six_paths();
    let stacked = Definitions::new().entry(["col1"], ["x"]);
    let fallback = Fallback::new(&stacked, &table);
    assert_eq!(lines(&fallback, 40), ["  col1", "    x"]);
    assert_eq!(
        fallback.measure(&Context::new(40)),
        stacked.measure(&Context::new(40))
    );
    assert_eq!(lines(&fallback, 200), lines(&table, 200));
    assert_eq!(
        fallback.measure(&Context::new(200)),
        table.measure(&Context::new(200))
    );
}

#[test]
fn a_short_column_keeps_its_width() {
    let table = Table::new([Column::new("n"), Column::new("title")])
        .row([Text::plain("abc"), Text::plain("x".repeat(60))]);
    let rendered = lines(&table, 30);
    assert!(widest(&rendered) <= 30, "{rendered:?}");
    assert_eq!(grid_columns(&rendered)[0], 3, "{rendered:?}");
}

#[test]
fn a_column_that_wraps_at_its_floor_is_not_narrowed_below_it() {
    let table = Table::new([Column::new("a"), Column::new("b")])
        .frame(Frame::Plain)
        .row(cells(["abc def ghi jkl", "x"]));
    assert_eq!(lines(&table, 11), ["a         b", "abc def   x", "ghi jkl"]);
}

#[test]
fn a_header_wider_than_the_floor_never_wraps_mid_word() {
    let table = Table::new([Column::new("a-long-header"), Column::new("b")])
        .frame(Frame::Plain)
        .row(cells(["aa bb cc dd ee ff gg hh ii jj", "y"]));
    let rendered = lines(&table, 16);
    assert_eq!(rendered[0], "a-long-header  b", "{rendered:?}");
}

#[test]
fn a_floor_of_zero_lets_an_ellipsis_column_narrow_to_one_cell() {
    let table = paths().frame(Frame::Plain).column_floor(0);
    let rendered = lines(&table, 33);
    assert_eq!(
        rendered[1], "crates/ctl-core/src/renders.rs  …",
        "{rendered:?}"
    );
}

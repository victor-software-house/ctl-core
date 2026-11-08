//! The frames ctl-core draws today, reproduced exactly on a small table.

use ctl_layout::{Column, Frame, Record, RecordStyle, RowSeparation, Table};
use ctl_text::{BoxChars, Context, Measurement, Renderable, Text};

use crate::common::{cells, lines};

fn small(frame: Frame, separation: RowSeparation) -> Table {
    Table::new([Column::new("id"), Column::new("name")])
        .frame(frame)
        .row_separation(separation)
        .row(cells(["1", "ab"]))
        .row(cells(["22", "c"]))
}

#[test]
fn a_grid_has_an_outer_frame_and_a_header_rule() {
    assert_eq!(
        lines(&small(Frame::Grid, RowSeparation::None), 80),
        [
            "┌────┬──────┐",
            "│ id ┆ name │",
            "╞════╪══════╡",
            "│ 1  ┆ ab   │",
            "│ 22 ┆ c    │",
            "└────┴──────┘",
        ]
    );
}

#[test]
fn a_grid_can_draw_a_rule_between_rows() {
    assert_eq!(
        lines(&small(Frame::Grid, RowSeparation::Rule), 80),
        [
            "┌────┬──────┐",
            "│ id ┆ name │",
            "╞════╪══════╡",
            "│ 1  ┆ ab   │",
            "├╌╌╌╌┼╌╌╌╌╌╌┤",
            "│ 22 ┆ c    │",
            "└────┴──────┘",
        ]
    );
}

#[test]
fn a_grid_can_leave_a_blank_row_between_rows() {
    assert_eq!(
        lines(&small(Frame::Grid, RowSeparation::Blank), 80),
        [
            "┌────┬──────┐",
            "│ id ┆ name │",
            "╞════╪══════╡",
            "│ 1  ┆ ab   │",
            "│    ┆      │",
            "│ 22 ┆ c    │",
            "└────┴──────┘",
        ]
    );
}

#[test]
fn a_header_rule_list_draws_one_rule_and_no_verticals() {
    assert_eq!(
        lines(&small(Frame::HeaderRule, RowSeparation::None), 80),
        ["id  name", "────────", "1   ab", "22  c"]
    );
}

#[test]
fn a_plain_list_draws_no_rule() {
    assert_eq!(
        lines(&small(Frame::Plain, RowSeparation::None), 80),
        ["id  name", "1   ab", "22  c"]
    );
}

#[test]
fn a_plain_list_can_draw_a_rule_between_rows() {
    assert_eq!(
        lines(&small(Frame::Plain, RowSeparation::Rule), 80),
        ["id  name", "1   ab", "────────", "22  c"]
    );
}

#[test]
fn a_plain_list_can_leave_a_blank_line_between_rows() {
    assert_eq!(
        lines(&small(Frame::Plain, RowSeparation::Blank), 80),
        ["id  name", "1   ab", "", "22  c"]
    );
}

#[test]
fn a_header_rule_list_can_also_rule_between_rows() {
    assert_eq!(
        lines(&small(Frame::HeaderRule, RowSeparation::Rule), 80),
        ["id  name", "────────", "1   ab", "────────", "22  c"]
    );
}

#[test]
fn ascii_box_characters_replace_the_unicode_ones() {
    let context = Context::new(80).with_box_chars(BoxChars::Ascii);
    let rendered: Vec<String> = small(Frame::Grid, RowSeparation::Rule)
        .render(&context)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        rendered,
        [
            "+----+------+",
            "| id | name |",
            "+====+======+",
            "| 1  | ab   |",
            "+----+------+",
            "| 22 | c    |",
            "+----+------+",
        ]
    );
}

#[test]
fn a_table_without_a_header_draws_no_header_row_or_rule() {
    let table = Table::new([Column::untitled(), Column::untitled()])
        .frame(Frame::HeaderRule)
        .row(cells(["1", "ab"]));
    assert_eq!(lines(&table, 80), ["1  ab"]);
}

#[test]
fn a_numeric_column_aligns_right_with_its_header() {
    let table = Table::new([Column::new("item"), Column::new("qty").numeric()])
        .frame(Frame::Plain)
        .row(cells(["a", "3"]))
        .row(cells(["bb", "120"]));
    assert_eq!(lines(&table, 80), ["item  qty", "a       3", "bb    120"]);
}

#[test]
fn a_short_row_gets_empty_cells_and_a_long_row_loses_its_extra() {
    let table = Table::new([Column::new("a"), Column::new("b")])
        .frame(Frame::Plain)
        .row(cells(["1"]))
        .row(cells(["2", "3", "4"]));
    assert_eq!(lines(&table, 80), ["a  b", "1", "2  3"]);
}

#[test]
fn a_borderless_record_right_aligns_keys() {
    assert_eq!(
        lines(&crate::common::ledger(), 80),
        ["ledger  tasks.yaml", "active  QCTL-014"]
    );
}

#[test]
fn a_left_aligned_record_pads_short_keys_on_the_right() {
    let record = Record::new()
        .style(RecordStyle::KeysLeft)
        .row(Text::plain("id"), Text::plain("QCTL-014"))
        .row(Text::plain("outcome"), Text::plain("done"));
    assert_eq!(lines(&record, 80), ["id       QCTL-014", "outcome  done"]);
}

#[test]
fn a_boxed_record_draws_a_full_box() {
    let record = Record::new()
        .style(RecordStyle::Boxed)
        .row(Text::plain("a"), Text::plain("b"))
        .row(Text::plain("cc"), Text::plain("d"));
    assert_eq!(
        lines(&record, 80),
        ["┌────┬───┐", "│ a  ┆ b │", "│ cc ┆ d │", "└────┴───┘"]
    );
}

#[test]
fn an_empty_boxed_record_draws_no_box() {
    let record = Record::new().style(RecordStyle::Boxed);
    assert!(lines(&record, 80).is_empty());
    assert_eq!(record.measure(&Context::new(80)), Measurement::exact(0));
}

#[test]
fn changing_a_records_style_keeps_its_rows() {
    let record = Record::new()
        .row(Text::plain("a"), Text::plain("b"))
        .style(RecordStyle::Boxed);
    assert_eq!(lines(&record, 80), ["┌───┬───┐", "│ a ┆ b │", "└───┴───┘"]);
}

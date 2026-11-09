//! Reviewed snapshots of every widget at widths 20, 40, 80, and 120. Each
//! snapshot is the plain text of the rendered lines, one file per width, in
//! `tests/snapshots`.

use ctl_layout::{Frame, Indent, RecordStyle, RowSeparation, Rule, Stack};
use ctl_text::{BoxChars, Context, Renderable, Text};

use crate::common::{flags, ledger, paths, queue, releases};

const WIDTHS: [u16; 4] = [20, 40, 80, 120];

fn check(name: &str, widget: &dyn Renderable, box_chars: BoxChars) {
    for width in WIDTHS {
        let context = Context::new(width).using_box_chars(box_chars);
        let lines: Vec<String> = widget
            .render(&context)
            .iter()
            .map(ToString::to_string)
            .collect();
        let rendered = format!("{}\n", lines.join("\n"));
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_path("../snapshots");
        settings.set_prepend_module_to_snapshot(false);
        settings.set_omit_expression(true);
        settings.bind(|| insta::assert_snapshot!(format!("{name}_{width}"), rendered));
    }
}

fn unicode(name: &str, widget: &dyn Renderable) {
    check(name, widget, BoxChars::Unicode);
}

#[test]
fn grid() {
    unicode("grid", &queue(Frame::Grid, RowSeparation::None));
}

#[test]
fn grid_with_row_rules() {
    unicode("grid_row_rule", &queue(Frame::Grid, RowSeparation::Rule));
}

#[test]
fn grid_with_blank_rows() {
    unicode("grid_blank_rows", &queue(Frame::Grid, RowSeparation::Blank));
}

#[test]
fn ascii_grid() {
    check(
        "ascii_grid",
        &queue(Frame::Grid, RowSeparation::Rule),
        BoxChars::Ascii,
    );
}

#[test]
fn header_rule() {
    unicode(
        "header_rule",
        &queue(Frame::HeaderRule, RowSeparation::None),
    );
}

#[test]
fn header_rule_with_row_rules() {
    unicode(
        "header_rule_row_rule",
        &queue(Frame::HeaderRule, RowSeparation::Rule),
    );
}

#[test]
fn plain() {
    unicode("plain", &queue(Frame::Plain, RowSeparation::None));
}

#[test]
fn plain_with_row_rules() {
    unicode("plain_row_rule", &queue(Frame::Plain, RowSeparation::Rule));
}

#[test]
fn plain_with_blank_rows() {
    unicode(
        "plain_blank_rows",
        &queue(Frame::Plain, RowSeparation::Blank),
    );
}

#[test]
fn ellipsis_column_beside_a_path() {
    unicode("ellipsis_path", &paths());
}

#[test]
fn a_url_in_a_grid() {
    unicode("grid_url", &releases());
}

#[test]
fn a_url_in_a_plain_list() {
    unicode("plain_url", &releases().frame(Frame::Plain));
}

#[test]
fn stacked_flags() {
    unicode("stacked_flags", &flags());
}

#[test]
fn record_with_right_keys() {
    unicode("record_keys_right", &ledger());
}

#[test]
fn record_with_left_keys() {
    unicode("record_keys_left", &ledger().style(RecordStyle::KeysLeft));
}

#[test]
fn boxed_record() {
    unicode("record_boxed", &ledger().style(RecordStyle::Boxed));
}

#[test]
fn rule() {
    unicode("rule", &Rule::new());
}

#[test]
fn titled_rule() {
    unicode(
        "rule_titled",
        &Rule::titled(Text::plain("Let a ledger declare row separation")),
    );
}

#[test]
fn stack_of_a_rule_a_record_and_a_table() {
    let stack = Stack::new()
        .spacing(1)
        .push(Rule::titled(Text::plain("Ledger")))
        .push(ledger())
        .push(queue(Frame::HeaderRule, RowSeparation::None));
    unicode("stack", &stack);
}

#[test]
fn indented_table() {
    unicode(
        "indent",
        &Indent::new(4, queue(Frame::Plain, RowSeparation::None)),
    );
}

//! A table never narrows a column below its floor, and a table that cannot fit
//! at its floors stacks, or wraps when the table fit says so.
//!
//! The list at each width and style is a reviewed snapshot in
//! `tests/snapshots`. Edition 2024 makes `set_var` unsafe, so each case that
//! reads a variable reruns this test binary as a child.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "app")]

use std::io::{self, Write as _};

use ctl_core::prelude::*;

#[path = "support/child.rs"]
mod child;

const CHILD: &str = "CTL_CORE_TABLE_FIT_CHILD";

const STYLES: [(&str, Frame); 3] = [
    ("grid", Frame::Grid),
    ("header_rule", Frame::HeaderRule),
    ("plain", Frame::Plain),
];

fn render(table: Table, width: u16, options: &RenderOptions) -> String {
    Document::new()
        .table(table)
        .render(options.width(width).fallback_width(None))
}

/// The options with no variable of the environment in play.
fn options() -> RenderOptions {
    RenderOptions::new(ColorMode::Never)
        .frame(Frame::Grid)
        .record_style(RecordStyle::KeysRight)
        .row_separation(RowSeparation::None)
        .column_floor(8)
        .table_fit(TableFit::Stack)
}

fn six_columns() -> Table {
    Table::new(["id", "name", "path", "status", "owner", "notes"])
        .id_column(0)
        .row([
            "QCTL-014",
            "ledger",
            "crates/ctl-core/src/render.rs",
            "queued",
            "alice",
            "Declare row separation",
        ])
        .row([
            "QCTL-015",
            "yamled",
            "crates/yamled/src/location.rs",
            "active",
            "alice",
            "Write a ledger atomically",
        ])
}

fn headerless() -> Table {
    Table::plain().row([
        "QCTL-014",
        "ledger-core",
        "crates/ctl-core/src/render.rs",
        "queued",
        "example-organisation",
        "Declare row separation",
    ])
}

#[test]
fn a_six_column_list_at_each_width_and_style() {
    for (style, frame) in STYLES {
        for width in [40, 80, 160] {
            let rendered = render(six_columns(), width, &options().frame(frame));
            insta::assert_snapshot!(format!("six_columns_{style}_{width}"), rendered);
        }
    }
}

#[test]
fn a_headerless_table_stacks_as_labels_over_descriptions() {
    insta::assert_snapshot!("headerless_40", render(headerless(), 40, &options()));
}

#[test]
fn a_help_table_stacked_below_its_threshold_keeps_its_form() {
    let table = Table::plain()
        .stacked_below(64, 1)
        .row(["-f", "--format", "Output representation"])
        .row(["-q", "--quiet", "Print nothing on success"]);
    insta::assert_snapshot!("help_stacked_40", render(table, 40, &options()));
}

#[test]
fn a_table_with_no_rows_draws_its_header_at_any_width() {
    let table = Table::new(["id", "name", "path", "status", "owner", "notes"]);
    let rendered = render(table, 40, &options());
    assert_eq!(rendered.lines().count(), 3, "{rendered}");
    assert!(rendered.starts_with("┌"), "{rendered}");
    assert!(rendered.contains("│ id"), "{rendered}");
}

fn long_title() -> Table {
    Table::new(["path", "title"]).ellipsis_column(1).row([
        "crates/ctl-core/src/render.rs",
        "A very long title for the row",
    ])
}

#[test]
fn a_floor_of_zero_narrows_an_ellipsis_column_to_one_cell() {
    let rendered = render(
        long_title(),
        32,
        &options().frame(Frame::Plain).column_floor(0),
    );
    let lines: Vec<&str> = rendered.lines().collect();
    assert_eq!(
        lines,
        [
            format!("{:<29}  …", "path").as_str(),
            "crates/ctl-core/src/render.rs  …"
        ]
    );
}

#[test]
fn the_default_floor_stacks_a_table_whose_ellipsis_column_would_narrow_below_it() {
    let rendered = render(long_title(), 32, &options().frame(Frame::Plain));
    let lines: Vec<&str> = rendered.lines().collect();
    assert_eq!(lines[0], "crates/ctl-core/src/render.rs", "{rendered}");
    assert!(lines[1].starts_with("title  A very long"), "{rendered}");
}

fn two_columns() -> Table {
    Table::new(["a", "b"]).row(["aaaa bbbb cccc dddd eeee ffff"; 2])
}

#[test]
fn a_floor_sets_the_narrowest_wrapped_column() {
    let wrap = options().frame(Frame::Plain).table_fit(TableFit::Wrap);
    let eight = render(two_columns(), 20, &wrap);
    assert_eq!(
        eight.lines().nth(1),
        Some("aaaa bbbb  aaaa bbbb"),
        "{eight}"
    );
    let twelve = render(two_columns(), 20, &wrap.column_floor(12));
    assert_eq!(
        twelve.lines().nth(1),
        Some("aaaa bbbb     aaaa bbbb"),
        "{twelve}"
    );
}

#[test]
fn a_table_beats_the_options_on_how_it_fits() {
    let narrow = options().frame(Frame::Plain).column_floor(12);
    let stacked = render(two_columns(), 20, &narrow.table_fit(TableFit::Stack));
    let wrapped = render(
        two_columns().stacking(TableFit::Wrap),
        20,
        &narrow.table_fit(TableFit::Stack),
    );
    assert!(stacked.starts_with("aaaa bbbb"), "{stacked}");
    assert!(stacked.contains("b  aaaa bbbb"), "{stacked}");
    assert!(wrapped.starts_with("a "), "{wrapped}");
    let restacked = render(
        two_columns().stacking(TableFit::Stack),
        20,
        &narrow.table_fit(TableFit::Wrap),
    );
    assert_eq!(restacked, stacked);
}

fn written(document: &str) {
    io::stdout()
        .write_all(document.as_bytes())
        .unwrap_or_else(|error| panic!("write the child's output: {error}"));
}

/// The options of a case that leaves the floor and the fit to the
/// environment.
fn open() -> RenderOptions {
    RenderOptions::new(ColorMode::Never)
        .frame(Frame::Grid)
        .record_style(RecordStyle::KeysRight)
        .row_separation(RowSeparation::None)
}

#[test]
fn the_environment_picks_wrap_and_a_framed_grid_runs_past_the_width_aligned() {
    let Some(output) = child::in_child(
        CHILD,
        "the_environment_picks_wrap_and_a_framed_grid_runs_past_the_width_aligned",
        &[("CTL_CORE_TABLE_FIT", "wrap")],
    ) else {
        child::case(|| written(&render(six_columns(), 40, &open())));
        return;
    };
    let rendered = child::case_stdout(&output);
    let widths: Vec<usize> = rendered.lines().map(|line| line.chars().count()).collect();
    // Frame 19, and the floors 8, 6, 8, 6, 5, 8.
    assert!(widths.iter().all(|width| *width == 60), "{rendered}");
}

#[test]
fn the_environment_picks_the_column_floor() {
    let Some(output) = child::in_child(
        CHILD,
        "the_environment_picks_the_column_floor",
        &[
            ("CTL_CORE_TABLE_FIT", "wrap"),
            ("CTL_CORE_COLUMN_FLOOR", "12"),
        ],
    ) else {
        child::case(|| written(&render(two_columns(), 20, &open().frame(Frame::Plain))));
        return;
    };
    assert_eq!(
        child::case_stdout(&output).lines().nth(1),
        Some("aaaa bbbb     aaaa bbbb")
    );
}

#[test]
fn the_environment_stacks_by_default() {
    let Some(output) = child::in_child(
        CHILD,
        "the_environment_stacks_by_default",
        &[("CTL_CORE_TABLE_FIT", "")],
    ) else {
        child::case(|| written(&render(six_columns(), 40, &open())));
        return;
    };
    assert!(
        child::case_stdout(&output).starts_with("QCTL-014\n"),
        "{}",
        child::case_stdout(&output)
    );
}

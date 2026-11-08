//! Helpers and fixtures shared by the test modules.

use ctl_layout::{Column, Frame, Record, RowSeparation, Table};
use ctl_text::{Context, Overflow, Renderable, Text};

/// The lines `widget` renders at `width`, as plain strings.
pub(crate) fn lines(widget: &dyn Renderable, width: u16) -> Vec<String> {
    widget
        .render(&Context::new(width))
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The cells of one row.
pub(crate) fn cells<const N: usize>(values: [&str; N]) -> [Text; N] {
    values.map(Text::plain)
}

/// The width of the widest of `lines`, in cells.
pub(crate) fn widest(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| ctl_text::text_width(line))
        .map(usize::from)
        .max()
        .unwrap_or(0)
}

/// An id, a title, and a numeric age: the queue a ctl CLI lists.
pub(crate) fn queue(frame: Frame, separation: RowSeparation) -> Table {
    Table::new([
        Column::new("id"),
        Column::new("title"),
        Column::new("age").numeric(),
    ])
    .frame(frame)
    .row_separation(separation)
    .row(cells([
        "QCTL-014",
        "Let a ledger declare row separation",
        "3",
    ]))
    .row(cells(["QCTL-015", "Write a ledger atomically", "120"]))
    .row(cells(["QCTL-016", "Name the next startable row", "45"]))
}

/// A path that must stay whole beside a title that may be cut.
pub(crate) fn paths() -> Table {
    Table::new([
        Column::new("path"),
        Column::new("title").overflow(Overflow::Ellipsis),
    ])
    .row(cells([
        "crates/ctl-core/src/renders.rs",
        "Let a ledger declare row separation",
    ]))
    .row(cells(["crates/ctl-text/src/wrap.rs", "ok"]))
}

/// A release URL in a grid.
pub(crate) fn releases() -> Table {
    let url = format!("https://example.test/{}", "a".repeat(39));
    Table::new([Column::new("id"), Column::new("url")])
        .row([Text::plain("QCTL-014"), Text::plain(url)])
}

/// The flags of a help screen, which stacks below 64 cells.
pub(crate) fn flags() -> Table {
    Table::new([Column::untitled(), Column::untitled(), Column::untitled()])
        .frame(Frame::Plain)
        .stacked_below(64, 2)
        .row(cells(["-f", "--format", "Output representation"]))
        .row(cells(["-q", "--quiet", "Print nothing on success"]))
}

/// Two labelled values.
pub(crate) fn ledger() -> Record {
    Record::new()
        .row(Text::plain("ledger"), Text::plain("tasks.yaml"))
        .row(Text::plain("active"), Text::plain("QCTL-014"))
}

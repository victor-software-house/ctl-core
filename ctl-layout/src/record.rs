//! A record: key and value rows.

use ctl_text::{Context, Line, Measurement, Renderable, Text};

use crate::align::Align;
use crate::column::Column;
use crate::frame::{Frame, RecordStyle};
use crate::table::Table;

/// Labelled values, one pair to a row:
///
/// ```text
/// version  1.4.0
/// channel  stable
/// ```
///
/// A record is a two-column [`Table`] with no header. It shares the table's
/// column widths, so a long value wraps under its own column and a long word
/// keeps its line. A record with no rows draws nothing.
#[derive(Clone, Debug)]
pub struct Record {
    table: Table,
}

impl Record {
    /// A record with no rows, with keys right-aligned and no frame.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws the record in `style`.
    #[must_use]
    pub fn style(self, style: RecordStyle) -> Self {
        Self {
            table: Self::table(style).rows(self.table.into_rows()),
        }
    }

    /// Adds `key` and its `value` as the last row. Either may span several
    /// lines, and the row is as tall as the taller of the two.
    #[must_use]
    pub fn row(self, key: impl Into<Text>, value: impl Into<Text>) -> Self {
        self.push(key.into(), value.into())
    }

    fn push(mut self, key: Text, value: Text) -> Self {
        self.table = self.table.row([key, value]);
        self
    }

    fn table(style: RecordStyle) -> Table {
        let (frame, keys) = match style {
            RecordStyle::KeysRight => (Frame::Plain, Align::Right),
            RecordStyle::KeysLeft => (Frame::Plain, Align::Left),
            RecordStyle::Boxed => (Frame::Grid, Align::Left),
        };
        Table::new([Column::untitled().align(keys), Column::untitled()]).frame(frame)
    }
}

impl Default for Record {
    fn default() -> Self {
        Self {
            table: Self::table(RecordStyle::default()),
        }
    }
}

impl Renderable for Record {
    fn measure(&self, context: &Context) -> Measurement {
        self.table.measure(context)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.table.render(context)
    }
}

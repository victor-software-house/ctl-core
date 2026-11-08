//! A record: key and value rows.

use ctl_text::{Context, Line, Measurement, Renderable, Text};

use crate::align::Align;
use crate::column::Column;
use crate::frame::{Frame, RecordStyle};
use crate::table::Table;

/// Labelled values, one pair to a row:
///
/// ```text
/// ledger  tasks.yaml
/// active  QCTL-014
/// ```
///
/// A record is a two-column [`Table`] with no header. It shares the table's
/// column widths, so a long value wraps under its own column and a long word
/// keeps its line.
#[derive(Clone, Debug, Default)]
pub struct Record {
    rows: Vec<(Text, Text)>,
    style: RecordStyle,
}

impl Record {
    /// A record with no rows, with keys right-aligned and no frame.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws the record in `style`.
    #[must_use]
    pub fn style(mut self, style: RecordStyle) -> Self {
        self.style = style;
        self
    }

    /// Adds a row.
    #[must_use]
    pub fn row(mut self, key: Text, value: Text) -> Self {
        self.rows.push((key, value));
        self
    }

    fn table(&self) -> Table {
        let (frame, keys) = match self.style {
            RecordStyle::KeysRight => (Frame::Plain, Align::Right),
            RecordStyle::KeysLeft => (Frame::Plain, Align::Left),
            RecordStyle::Boxed => (Frame::Grid, Align::Left),
        };
        Table::new([Column::untitled().align(keys), Column::untitled()])
            .frame(frame)
            .rows(
                self.rows
                    .iter()
                    .map(|(key, value)| [key.clone(), value.clone()]),
            )
    }
}

impl Renderable for Record {
    fn measure(&self, context: &Context) -> Measurement {
        self.table().measure(context)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.table().render(context)
    }
}

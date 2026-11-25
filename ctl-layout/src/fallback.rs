//! A table, or another widget when the table does not fit.

use std::borrow::Borrow;

use ctl_text::{Context, Line, Measurement, Renderable};

use crate::table::Table;

/// A [`Table`], or another widget when the table does not fit.
///
/// It is the fit counterpart of [`Breakpoint`](crate::Breakpoint). A
/// breakpoint chooses by a width the caller guesses, and a fallback by
/// [`Table::fits`]: the table when its columns at their floors and its frame
/// fit the context's width, and `stacked` when they do not. It measures and
/// renders whichever applies, so a table gives way to rows drawn as records
/// exactly when it would run past the width. The table is owned or borrowed:
/// `Fallback::new(&stacked, &table)` works.
///
/// ```
/// use ctl_layout::{Column, Context, Definitions, Fallback, Frame, Renderable, Table};
///
/// let table = Table::new([Column::new("id"), Column::new("title")])
///     .frame(Frame::Plain)
///     .row(["QCTL-014", "Write a log"]);
/// let stacked = Definitions::new().entry(["QCTL-014"], ["Write a log"]);
/// let widget = Fallback::new(stacked, table);
///
/// let at = |width| -> Vec<String> {
///     widget
///         .render(&Context::new(width))
///         .iter()
///         .map(ToString::to_string)
///         .collect()
/// };
/// assert_eq!(at(40), ["id        title", "QCTL-014  Write a log"]);
/// assert_eq!(at(17), ["  QCTL-014", "    Write a log"]);
/// ```
#[derive(Clone, Debug)]
pub struct Fallback<S, T> {
    stacked: S,
    table: T,
}

impl<S: Renderable, T: Borrow<Table>> Fallback<S, T> {
    /// `table` when it fits the width, `stacked` when it does not.
    #[must_use]
    pub fn new(stacked: S, table: T) -> Self {
        Self { stacked, table }
    }

    fn pick(&self, context: &Context) -> &dyn Renderable {
        let table: &Table = self.table.borrow();
        if table.fits(context) {
            table
        } else {
            &self.stacked
        }
    }
}

impl<S: Renderable, T: Borrow<Table>> Renderable for Fallback<S, T> {
    fn measure(&self, context: &Context) -> Measurement {
        self.pick(context).measure(context)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.pick(context).render(context)
    }
}

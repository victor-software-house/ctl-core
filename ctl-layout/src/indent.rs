//! A widget moved right.

use std::fmt;

use ctl_text::{Context, Line, Measurement, Renderable};

use crate::lines::indent_lines;

/// A widget moved right by a number of cells. The child gets the width that
/// is left, and a blank line stays blank.
pub struct Indent {
    columns: u16,
    child: Box<dyn Renderable>,
}

impl Indent {
    /// `child` moved right by `columns` cells.
    #[must_use]
    pub fn new(columns: u16, child: impl Renderable + 'static) -> Self {
        Self::boxed(columns, Box::new(child))
    }

    /// An already boxed `child` moved right by `columns` cells.
    #[must_use]
    pub fn boxed(columns: u16, child: Box<dyn Renderable>) -> Self {
        Self { columns, child }
    }

    fn inner(&self, context: Context) -> Context {
        context.with_width(context.width().saturating_sub(self.columns).max(1))
    }
}

impl fmt::Debug for Indent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Indent")
            .field("columns", &self.columns)
            .finish_non_exhaustive()
    }
}

impl Renderable for Indent {
    fn measure(&self, context: &Context) -> Measurement {
        let measured = self.child.measure(&self.inner(*context));
        Measurement::new(
            measured.minimum().saturating_add(self.columns),
            measured.maximum().saturating_add(self.columns),
        )
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        indent_lines(self.child.render(&self.inner(*context)), self.columns)
    }
}

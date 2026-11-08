//! Widgets one above the other.

use std::fmt;

use ctl_text::{Context, Line, Measurement, Renderable};

/// Widgets one above the other, each rendered at the full width.
///
/// A child that renders no lines is skipped, so it leaves no gap.
#[derive(Default)]
pub struct Stack {
    children: Vec<Box<dyn Renderable>>,
    spacing: u16,
}

impl Stack {
    /// A stack with no children and no space between them.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Leaves `lines` blank lines between consecutive children.
    #[must_use]
    pub fn spacing(mut self, lines: u16) -> Self {
        self.spacing = lines;
        self
    }

    /// Adds `child` below the others.
    #[must_use]
    pub fn push(self, child: impl Renderable + 'static) -> Self {
        self.push_boxed(Box::new(child))
    }

    /// Adds an already boxed `child` below the others.
    #[must_use]
    pub fn push_boxed(mut self, child: Box<dyn Renderable>) -> Self {
        self.children.push(child);
        self
    }
}

impl fmt::Debug for Stack {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Stack")
            .field("children", &self.children.len())
            .field("spacing", &self.spacing)
            .finish()
    }
}

impl Renderable for Stack {
    fn measure(&self, context: &Context) -> Measurement {
        self.children
            .iter()
            .map(|child| child.measure(context))
            .fold(Measurement::exact(0), |widest, child| {
                Measurement::new(
                    widest.minimum().max(child.minimum()),
                    widest.maximum().max(child.maximum()),
                )
            })
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        let mut lines: Vec<Line> = Vec::new();
        for child in &self.children {
            let rendered = child.render(context);
            if rendered.is_empty() {
                continue;
            }
            if !lines.is_empty() {
                lines.extend((0..self.spacing).map(|_| Line::new()));
            }
            lines.extend(rendered);
        }
        lines
    }
}

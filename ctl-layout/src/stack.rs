//! Widgets one above the other.

use std::fmt;

use ctl_text::{Context, Line, Measurement, Renderable};

/// Widgets one above the other, each rendered at the full width.
///
/// A child whose lines are all empty is skipped, so it leaves no gap. A child
/// may be borrowed, as in `Stack::new().push(&widget)`, so a widget drawn in
/// two places needs no clone.
#[derive(Default)]
pub struct Stack<'a> {
    children: Vec<Box<dyn Renderable + 'a>>,
    spacing: u16,
}

impl<'a> Stack<'a> {
    /// A stack with no children and no space between them.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Leaves `lines` blank lines between consecutive children that render
    /// something. The default is none.
    #[must_use]
    pub fn spacing(mut self, lines: u16) -> Self {
        self.spacing = lines;
        self
    }

    /// Adds `child` below the others.
    #[must_use]
    pub fn push(mut self, child: impl Renderable + 'a) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl fmt::Debug for Stack<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Stack")
            .field("children", &self.children.len())
            .field("spacing", &self.spacing)
            .finish()
    }
}

impl Renderable for Stack<'_> {
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
            if rendered.iter().all(Line::is_empty) {
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

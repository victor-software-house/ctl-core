//! One widget below a width and another from it up.

use ctl_text::{Context, Line, Measurement, Renderable};

/// One widget below a width and another from it up.
///
/// It measures and renders whichever widget applies at the context's width,
/// so a table can give way to [`Definitions`](crate::Definitions) on a narrow
/// terminal. Both widgets hold their own copy of the data they draw.
#[derive(Clone, Debug)]
pub struct Breakpoint<N, W> {
    threshold: u16,
    narrow: N,
    wide: W,
}

impl<N: Renderable, W: Renderable> Breakpoint<N, W> {
    /// `narrow` when the width is under `threshold` cells, `wide` at
    /// `threshold` and above.
    #[must_use]
    pub fn new(threshold: u16, narrow: N, wide: W) -> Self {
        Self {
            threshold,
            narrow,
            wide,
        }
    }

    fn pick(&self, context: &Context) -> &dyn Renderable {
        if context.width() < self.threshold {
            &self.narrow
        } else {
            &self.wide
        }
    }
}

impl<N: Renderable, W: Renderable> Renderable for Breakpoint<N, W> {
    fn measure(&self, context: &Context) -> Measurement {
        self.pick(context).measure(context)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.pick(context).render(context)
    }
}

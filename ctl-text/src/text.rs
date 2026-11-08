//! Styled text that spans lines.

use anstyle::Style;

use crate::line::Line;
use crate::protocol::{Context, Measurement, Renderable};
use crate::sanitize::{expand_tabs, strip_unsafe};
use crate::wrap::{self, Overflow};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Segment {
    text: String,
    style: Style,
}

/// Styled text of any length: a paragraph, a cell, a title.
///
/// Text from outside the program is safe to put here. Every character that
/// [`is_unsafe`](crate::verbatim::is_unsafe) refuses is removed, except
/// newline and tab. A newline starts a new line, and a tab expands to the next
/// tab stop of the [`Context`] it is rendered in, counted from the start of
/// its line.
///
/// `Text` is [`Renderable`]: [`Overflow`] decides how it fits a width.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Text {
    segments: Vec<Segment>,
    overflow: Overflow,
}

impl Text {
    /// Text with nothing in it. It renders to no lines.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `text` in the default style.
    #[must_use]
    pub fn plain(text: impl AsRef<str>) -> Self {
        Self::styled(text, Style::new())
    }

    /// `text` in `style`.
    #[must_use]
    pub fn styled(text: impl AsRef<str>, style: Style) -> Self {
        let mut created = Self::new();
        created.push(text, style);
        created
    }

    /// Adds `text` in `style` after what is already there. A newline in
    /// `text` starts a new line.
    pub fn push(&mut self, text: impl AsRef<str>, style: Style) {
        self.push_str(text.as_ref(), style);
    }

    fn push_str(&mut self, text: &str, style: Style) {
        self.segments.push(Segment {
            text: strip_unsafe(text, &['\n', '\t']),
            style,
        });
    }

    /// Sets what to do with a line wider than its width. The default is
    /// [`Overflow::KeepWords`].
    #[must_use]
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// The text as lines, with tabs expanded to multiples of `tab_width`.
    ///
    /// Nothing is wrapped. A text that ends in a newline ends in an empty
    /// line.
    #[must_use]
    pub fn lines(&self, tab_width: u16) -> Vec<Line> {
        let mut lines: Vec<Line> = Vec::new();
        for segment in &self.segments {
            for (index, part) in segment.text.split('\n').enumerate() {
                let mut line = if index == 0 {
                    lines.pop().unwrap_or_default()
                } else {
                    Line::new()
                };
                let expanded = expand_tabs(part, usize::from(line.width()), tab_width);
                line.append(&expanded, segment.style);
                lines.push(line);
            }
        }
        lines
    }

    /// The lines of the text with the spaces at the end of each removed.
    fn trimmed_lines(&self, context: Context) -> Vec<Line> {
        let mut lines = self.lines(context.tab_width());
        for line in &mut lines {
            line.trim_end();
        }
        lines
    }
}

impl Renderable for Text {
    fn measure(&self, context: &Context) -> Measurement {
        let lines = self.trimmed_lines(*context);
        let maximum = lines.iter().map(Line::width).max().unwrap_or(0);
        let minimum = lines
            .iter()
            .map(|line| wrap::minimum(line, self.overflow))
            .max()
            .unwrap_or(0);
        Measurement::new(minimum, maximum)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.trimmed_lines(*context)
            .iter()
            .flat_map(|line| line.fit(context.width(), self.overflow))
            .collect()
    }
}

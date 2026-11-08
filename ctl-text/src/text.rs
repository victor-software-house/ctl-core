//! Styled text that spans lines.

use std::num::NonZeroU16;

use anstyle::Style;

use crate::line::Line;
use crate::protocol::{Context, Measurement, Renderable};
use crate::sanitize::{Breaks, clean, expand_tabs};
use crate::span::Span;
use crate::wrap::Overflow;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Segment {
    text: String,
    style: Style,
}

/// Styled text of any length: a paragraph, a cell, a title.
///
/// Text from outside the program is safe to put here. Every character that
/// [`is_unsafe`](crate::unsafe_chars::is_unsafe) refuses is removed, except
/// newline and tab. Every other line break (carriage return, vertical tab,
/// form feed, next line, and the line and paragraph separators) becomes a
/// newline, and a carriage return before a line feed is one break with it. A
/// newline starts a new line, and a tab expands to the next tab stop of the
/// [`Context`] it is rendered in, counted from the start of its line.
///
/// Two texts are equal when their content, styles, and [`Overflow`] are:
/// empty pieces are dropped and neighbours in one style are joined, however
/// the text was pushed.
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
        self.push_segment(clean(text, Breaks::Newline), style);
    }

    /// Adds `text`, which is already clean, unless it is empty; joins the last
    /// segment when it has the same style.
    fn push_segment(&mut self, text: String, style: Style) {
        if text.is_empty() {
            return;
        }
        match self.segments.last_mut() {
            Some(last) if last.style == style => last.text.push_str(&text),
            Some(_) | None => self.segments.push(Segment { text, style }),
        }
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
    pub fn lines(&self, tab_width: NonZeroU16) -> Vec<Line> {
        let mut lines: Vec<Line> = Vec::new();
        for segment in &self.segments {
            for (index, part) in segment.text.split('\n').enumerate() {
                let mut line = if index == 0 {
                    lines.pop().unwrap_or_default()
                } else {
                    Line::new()
                };
                append_expanded(&mut line, part, segment.style, tab_width);
                lines.push(line);
            }
        }
        lines
    }

    /// The lines of the text with the spaces at the end of each removed.
    fn trimmed_lines(&self, context: &Context) -> Vec<Line> {
        let mut lines = self.lines(context.tab_width());
        for line in &mut lines {
            line.trim_end();
        }
        lines
    }
}

/// Adds `part`, which holds no newline, to `line`, with each tab expanded.
///
/// The text before the first tab goes in first, so the width a tab counts
/// from is the line's own: a cluster that the new text continues from the
/// line's last span, such as a variation selector after a heart, widens the
/// line before the tab is measured.
fn append_expanded(line: &mut Line, part: &str, style: Style, tab_width: NonZeroU16) {
    let (head, tabbed) = part.split_at(part.find('\t').unwrap_or(part.len()));
    line.append(head, style);
    if !tabbed.is_empty() {
        let column = usize::from(line.width());
        line.append(&expand_tabs(tabbed, column, tab_width), style);
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        Self::plain(text)
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self::plain(text)
    }
}

impl From<Span> for Text {
    fn from(span: Span) -> Self {
        let mut text = Self::new();
        text.push_segment(span.text, span.style);
        text
    }
}

impl Text {
    /// [`measure`](Renderable::measure) as if the text's overflow were
    /// `overflow`, so a container that decides overflow per column measures a
    /// cell without cloning it.
    #[must_use]
    pub fn measure_as(&self, context: &Context, overflow: Overflow) -> Measurement {
        let lines = self.trimmed_lines(context);
        let maximum = lines.iter().map(Line::width).max().unwrap_or(0);
        let minimum = lines
            .iter()
            .map(|line| line.minimum_width(overflow))
            .max()
            .unwrap_or(0);
        Measurement::new(minimum, maximum)
    }

    /// [`render`](Renderable::render) as if the text's overflow were
    /// `overflow`, so a container that decides overflow per column renders a
    /// cell without cloning it.
    #[must_use]
    pub fn render_as(&self, context: &Context, overflow: Overflow) -> Vec<Line> {
        self.trimmed_lines(context)
            .iter()
            .flat_map(|line| line.fit(context.width(), overflow))
            .collect()
    }
}

impl Renderable for Text {
    fn measure(&self, context: &Context) -> Measurement {
        self.measure_as(context, self.overflow)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.render_as(context, self.overflow)
    }
}

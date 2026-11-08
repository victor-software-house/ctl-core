//! One run of text in one style.

use anstyle::Style;

use crate::cells::{TAB_WIDTH, text_width};
use crate::sanitize::{expand_tabs, strip_unsafe};

/// A run of text in one style, on one line.
///
/// A span holds no character that [`is_unsafe`](crate::verbatim::is_unsafe)
/// refuses, so no escape sequence, directional override, or line separator
/// reaches a terminal through it. It holds no tab either. A cluster never
/// spans two spans, so a line's width is the sum of its spans' widths.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    pub(crate) text: String,
    pub(crate) style: Style,
}

impl Span {
    /// A span of `text` in `style`.
    ///
    /// Tabs expand to the next multiple of [`TAB_WIDTH`] cells, counted from
    /// the start of the span, which is right for a span that starts its line.
    /// Every unsafe character is removed, newline included: a span is one
    /// line. Build text that spans lines, or follows other spans on a line,
    /// with [`Text`](crate::Text), which splits at each newline and counts
    /// tab stops from the line start.
    #[must_use]
    pub fn new(text: impl AsRef<str>, style: Style) -> Self {
        Self::from_text(text.as_ref(), style)
    }

    fn from_text(text: &str, style: Style) -> Self {
        let safe = strip_unsafe(text, &['\t']);
        Self {
            text: expand_tabs(&safe, 0, TAB_WIDTH),
            style,
        }
    }

    /// The text, without escape sequences, tabs, or newlines.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The style the text is drawn in.
    #[must_use]
    pub fn style(&self) -> Style {
        self.style
    }

    /// The width of the text in terminal cells.
    #[must_use]
    pub fn width(&self) -> u16 {
        text_width(&self.text)
    }
}

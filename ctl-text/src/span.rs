//! One run of text in one style.

use std::fmt;

use anstyle::Style;

use crate::cells::text_width;
use crate::sanitize::{is_clean, span_text};

/// A run of text in one style, on one line.
///
/// A span holds no character that [`is_unsafe`](crate::unsafe_chars::is_unsafe)
/// refuses, so no escape sequence, directional override, or line separator
/// reaches a terminal through it. It holds no tab either. A cluster never
/// spans two spans, so a line's width is the sum of its spans' widths.
///
/// Its [`Display`](fmt::Display) prints the text without the style.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    pub(crate) text: String,
    pub(crate) style: Style,
}

impl Span {
    /// A span of `text` in `style`.
    ///
    /// Tabs expand to the next multiple of [`TAB_WIDTH`](crate::TAB_WIDTH)
    /// cells, counted from the start of the span. That is right for a span
    /// that starts its line.
    ///
    /// A span is one line, so every line break becomes a space, and the words
    /// around it stay apart. The line breaks are newline, carriage return,
    /// vertical tab, form feed, next line, and the line and paragraph
    /// separators. Every other unsafe character is removed.
    ///
    /// Build text that spans lines, or follows other spans on a line, with
    /// [`Text`](crate::Text). It splits at each newline and counts tab stops
    /// from the line start.
    ///
    /// To keep an owned `String` as the span's text when it needs no change,
    /// use `Span::from(string)`.
    #[must_use]
    pub fn new(text: impl AsRef<str>, style: Style) -> Self {
        let text = text.as_ref();
        let text = if is_clean(text) {
            text.to_owned()
        } else {
            span_text(text)
        };
        Self { text, style }
    }

    /// The text, without escape sequences, tabs, or line breaks.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The style the text is drawn in. Layout never changes it.
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

impl From<&str> for Span {
    fn from(text: &str) -> Self {
        Self::new(text, Style::new())
    }
}

impl From<String> for Span {
    fn from(text: String) -> Self {
        let text = if is_clean(&text) {
            text
        } else {
            span_text(&text)
        };
        Self {
            text,
            style: Style::new(),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

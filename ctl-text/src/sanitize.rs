//! The two steps that make outside text safe to lay out: remove the unsafe
//! characters and turn line breaks into something the layout understands, then
//! expand tabs.

use std::iter;
use std::num::NonZeroU16;

use crate::cells::{TAB_WIDTH, text_width};
use crate::unsafe_chars::{LINE_SEPARATOR, PARAGRAPH_SEPARATOR, is_unsafe};

/// U+0085, the next line control.
const NEXT_LINE: char = '\u{85}';

/// What [`clean`] makes of a line break.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Breaks {
    /// A span is one line: every break, newline included, becomes a space.
    Space,
    /// Text spans lines: every break becomes a newline.
    Newline,
}

impl Breaks {
    /// What a break other than a newline becomes.
    fn replacement(self) -> char {
        match self {
            Self::Space => ' ',
            Self::Newline => '\n',
        }
    }
}

/// `text` without the characters [`is_unsafe`] refuses, except tab.
///
/// A newline, carriage return, vertical tab, form feed, next line control, and
/// the line and paragraph separators are line breaks: `breaks` says what each
/// becomes, so the words around one stay apart. A carriage return before a
/// line feed is one break with it.
pub(crate) fn clean(text: &str, breaks: Breaks) -> String {
    let mut cleaned = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\t' => cleaned.push('\t'),
            '\r' => {
                let _paired = characters.next_if_eq(&'\n');
                cleaned.push(breaks.replacement());
            }
            '\n' | '\u{b}' | '\u{c}' | NEXT_LINE | LINE_SEPARATOR | PARAGRAPH_SEPARATOR => {
                cleaned.push(breaks.replacement());
            }
            other if is_unsafe(other) => {}
            other => cleaned.push(other),
        }
    }
    cleaned
}

/// Whether `text` holds no character [`is_unsafe`] refuses, tab and newline
/// included, so a span can keep it as it is.
pub(crate) fn is_clean(text: &str) -> bool {
    text.chars().all(|character| !is_unsafe(character))
}

/// The text of a span made from `text`: cleaned, with tabs expanded from
/// column 0.
pub(crate) fn span_text(text: &str) -> String {
    let cleaned = clean(text, Breaks::Space);
    if cleaned.contains('\t') {
        expand_tabs(&cleaned, 0, TAB_WIDTH)
    } else {
        cleaned
    }
}

/// `text` with each tab replaced by spaces up to the next multiple of
/// `tab_width`, counting from a line that already holds `column` cells.
///
/// `text` holds no newline. A tab ends a cluster on both sides, so the width
/// of each piece between tabs adds up exactly.
pub(crate) fn expand_tabs(text: &str, column: usize, tab_width: NonZeroU16) -> String {
    let tab = usize::from(tab_width.get());
    let mut expanded = String::with_capacity(text.len());
    let mut column = column;
    for (index, chunk) in text.split('\t').enumerate() {
        if index > 0 {
            let padding = tab - column % tab;
            expanded.extend(iter::repeat_n(' ', padding));
            column += padding;
        }
        expanded.push_str(chunk);
        column += usize::from(text_width(chunk));
    }
    expanded
}

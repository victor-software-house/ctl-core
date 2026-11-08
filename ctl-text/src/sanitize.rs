//! The two steps that make outside text safe to lay out: remove the unsafe
//! characters, then expand tabs.

use std::iter;

use crate::cells::text_width;
use crate::verbatim::is_unsafe;

/// `text` without the characters [`is_unsafe`] refuses, except those in
/// `keep`.
pub(crate) fn strip_unsafe(text: &str, keep: &[char]) -> String {
    text.chars()
        .filter(|character| keep.contains(character) || !is_unsafe(*character))
        .collect()
}

/// `text` with each tab replaced by spaces up to the next multiple of
/// `tab_width`, counting from a line that already holds `column` cells.
///
/// `text` holds no newline. A tab width of 0 counts as 1.
pub(crate) fn expand_tabs(text: &str, column: usize, tab_width: u16) -> String {
    let tab = usize::from(tab_width.max(1));
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

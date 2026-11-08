//! Terminal cell widths, counted per extended grapheme cluster.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The tab stop spacing used when a caller sets none: every 8 cells.
pub const TAB_WIDTH: u16 = 8;

/// The Unicode versions of the two data sets that decide a width.
///
/// A cluster's boundaries come from `unicode-segmentation` and its width from
/// `unicode-width`. The two crates ship their own tables, so they can sit on
/// different Unicode versions, and an emoji one knows and the other does not
/// is counted wrongly. `tests/it/unicode_version.rs` pins both, so a
/// dependency upgrade that moves either one fails until the pin is read and
/// updated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct UnicodeVersions {
    /// The version of the width tables in `unicode-width`.
    pub width: (u8, u8, u8),
    /// The version of the grapheme cluster tables in `unicode-segmentation`.
    pub segmentation: (u64, u64, u64),
}

/// The Unicode versions behind [`cluster_width`] and [`text_width`].
pub const UNICODE_VERSION: UnicodeVersions = UnicodeVersions {
    width: unicode_width::UNICODE_VERSION,
    segmentation: unicode_segmentation::UNICODE_VERSION,
};

/// The width in cells of `cluster`, taken as one extended grapheme cluster.
///
/// East Asian ambiguous characters count 1 cell. A ZWJ emoji sequence counts
/// as one emoji, 2 cells, not the sum of its parts. A width over `u16::MAX`
/// saturates.
#[must_use]
pub fn cluster_width(cluster: &str) -> u16 {
    u16::try_from(UnicodeWidthStr::width(cluster)).unwrap_or(u16::MAX)
}

/// The width in cells of `text`: the sum of [`cluster_width`] over its
/// extended grapheme clusters. A sum over `u16::MAX` saturates.
#[must_use]
pub fn text_width(text: &str) -> u16 {
    text.graphemes(true)
        .map(cluster_width)
        .fold(0, u16::saturating_add)
}

/// The byte offset where the longest prefix of whole clusters that fits in
/// `width` cells ends.
///
/// A cluster that would cross `width` stays out whole, so the prefix can be
/// narrower than `width`.
#[must_use]
pub fn prefix_len(text: &str, width: u16) -> usize {
    let mut used: u16 = 0;
    for (offset, cluster) in text.grapheme_indices(true) {
        used = used.saturating_add(cluster_width(cluster));
        if used > width {
            return offset;
        }
    }
    text.len()
}

/// Whether `character` may end a word: whitespace that is not a no-break
/// space (U+00A0, U+2007, U+202F).
pub(crate) fn is_break_space(character: char) -> bool {
    character.is_whitespace() && !matches!(character, '\u{a0}' | '\u{2007}' | '\u{202f}')
}

/// Whether every character of `cluster` may end a word.
pub(crate) fn is_space(cluster: &str) -> bool {
    cluster.chars().all(is_break_space)
}

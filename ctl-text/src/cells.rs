//! Terminal cell widths, counted per extended grapheme cluster.

use std::num::NonZeroU16;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The tab stop spacing used when a caller sets none: every 8 cells.
///
/// Built from the smallest non-zero value because a denied lint rules out
/// `NonZeroU16::new(8).unwrap()`.
pub const TAB_WIDTH: NonZeroU16 = NonZeroU16::MIN.saturating_add(7);

/// A Unicode version, such as 17.0.0.
///
/// The dependencies state theirs as bare tuples of different integer types;
/// this is the one shape the crate hands out.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnicodeVersion {
    /// The major version.
    pub major: u64,
    /// The minor version.
    pub minor: u64,
    /// The update version.
    pub update: u64,
}

/// The Unicode versions of the two data sets that decide a width.
///
/// A cluster's boundaries come from `unicode-segmentation` and its width from
/// `unicode-width`. The two crates ship their own tables, so they can sit on
/// different Unicode versions. An emoji that one knows and the other does not
/// is counted wrongly. This crate's tests pin both versions, so a dependency
/// upgrade that moves either one fails until the pin is read and updated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct UnicodeVersions {
    /// The version of the width tables in `unicode-width`.
    pub width: UnicodeVersion,
    /// The version of the grapheme cluster tables in `unicode-segmentation`.
    pub segmentation: UnicodeVersion,
}

/// A `u8` as a `u64`, because `From` cannot run in a constant.
const fn widen(value: u8) -> u64 {
    u64::from_le_bytes([value, 0, 0, 0, 0, 0, 0, 0])
}

/// The Unicode versions behind [`cluster_width`] and [`text_width`].
pub const UNICODE_VERSIONS: UnicodeVersions = UnicodeVersions {
    width: UnicodeVersion {
        major: widen(unicode_width::UNICODE_VERSION.0),
        minor: widen(unicode_width::UNICODE_VERSION.1),
        update: widen(unicode_width::UNICODE_VERSION.2),
    },
    segmentation: UnicodeVersion {
        major: unicode_segmentation::UNICODE_VERSION.0,
        minor: unicode_segmentation::UNICODE_VERSION.1,
        update: unicode_segmentation::UNICODE_VERSION.2,
    },
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
///
/// The sum is per cluster, so it can exceed the width a terminal gives a run
/// that shapes into one glyph across clusters. The Arabic lam followed by
/// alef is two clusters of 1 cell each, so 2, though a font may draw one
/// ligature.
#[must_use]
pub fn text_width(text: &str) -> u16 {
    text.graphemes(true)
        .map(cluster_width)
        .fold(0, u16::saturating_add)
}

/// The byte offset where the longest prefix of whole clusters that fits in
/// `width` cells ends.
///
/// Slice with it to cut `text` safely: `&text[..prefix_len(text, width)]`.
/// A cluster that would cross `width` stays out whole, so the prefix can be
/// narrower than `width`. When all of `text` fits, the offset is `text.len()`.
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

/// Whether `character` may end a word. That is whitespace other than a
/// no-break space (U+00A0, U+2007, U+202F), or the zero width space (U+200B).
/// The zero width space marks a place to break and shows nothing.
pub(crate) fn is_break_space(character: char) -> bool {
    (character.is_whitespace() && !matches!(character, '\u{a0}' | '\u{2007}' | '\u{202f}'))
        || character == '\u{200b}'
}

/// Whether every character of `cluster` may end a word.
pub(crate) fn is_space(cluster: &str) -> bool {
    cluster.chars().all(is_break_space)
}

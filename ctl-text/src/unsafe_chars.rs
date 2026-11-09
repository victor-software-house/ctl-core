//! Which characters are unsafe to print to a terminal as they are, and the
//! function that removes them.
//!
//! Text from outside the program can carry characters that act on the
//! terminal or on the layout instead of showing. A title, a commit subject,
//! or a file's contents can all hold them.
//!
//! A CLI that prints such text drops what [`is_unsafe`] refuses, with
//! [`strip_unsafe`]. `Document::verbatim` in ctl-core applies the same rule.
//! [`Span::new`](crate::Span::new) and [`Text`](crate::Text) apply it too,
//! but keep a line break as a space or a newline. The words around it then
//! stay apart.
//!
//! The set is three whole sets, each taken by its definition:
//!
//! 1. General category Cc, the C0 controls, DEL, and the C1 controls, 65
//!    characters, through [`char::is_control`]. A control can start an escape
//!    sequence that moves the cursor, rewrites the screen, or sets the
//!    clipboard.
//! 2. The 12 directional formatting characters of UAX #9, from
//!    [`unicode_bidi::format_chars`]: ALM, LRM, RLM, LRE, RLE, PDF, LRO, RLO,
//!    LRI, RLI, FSI, and PDI. They reorder what the reader sees, so the text
//!    shown differs from the text held.
//! 3. General categories Zl and Zp, one character each: [`LINE_SEPARATOR`] and
//!    [`PARAGRAPH_SEPARATOR`]. A terminal or a tool may break a line there that
//!    the layout counted as one.
//!
//! The set follows Unicode 16.0.0 and UAX #9 revision 50, which unicode-bidi
//! 0.3.18 implements. Cc, Zl, and Zp have had the same members since
//! Unicode 1.1.
//!
//! The other format characters (general category Cf) stay. They neither act on
//! the terminal nor reorder text, and text needs them.
//!
//! - ZWJ joins emoji.
//! - ZWNJ shapes Persian and Indic scripts.
//! - The tag characters spell subdivision flags.
//! - The soft hyphen marks a break.
//! - The Arabic number signs render as visible marks.

use unicode_bidi::format_chars::{ALM, FSI, LRE, LRI, LRM, LRO, PDF, PDI, RLE, RLI, RLM, RLO};

/// U+2028 LINE SEPARATOR, the only character in general category Zl.
/// [`is_unsafe`] refuses it.
pub const LINE_SEPARATOR: char = '\u{2028}';

/// U+2029 PARAGRAPH SEPARATOR, the only character in general category Zp.
/// [`is_unsafe`] refuses it.
pub const PARAGRAPH_SEPARATOR: char = '\u{2029}';

/// Whether `character` is in the set the [module](self) defines.
///
/// Newline and tab are controls, so they are refused too. A caller that
/// keeps them checks for them first, or passes [`Keep`] to [`strip_unsafe`].
#[must_use]
pub fn is_unsafe(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            ALM | LRM
                | RLM
                | LRE
                | RLE
                | PDF
                | LRO
                | RLO
                | LRI
                | RLI
                | FSI
                | PDI
                | LINE_SEPARATOR
                | PARAGRAPH_SEPARATOR
        )
}

/// The controls [`strip_unsafe`] leaves in place.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Keep {
    /// Nothing: every unsafe character goes, newline and tab included.
    Nothing,
    /// Newline and tab, the two controls that preformatted text needs.
    NewlineAndTab,
}

/// `text` without the characters [`is_unsafe`] refuses, except those `keep`
/// names.
///
/// A removed character leaves no space behind, so words on either side of a
/// removed line separator join. [`Text`](crate::Text) turns a line break into
/// a newline instead.
#[must_use]
pub fn strip_unsafe(text: &str, keep: Keep) -> String {
    text.chars()
        .filter(|character| match keep {
            Keep::Nothing => !is_unsafe(*character),
            Keep::NewlineAndTab => matches!(character, '\n' | '\t') || !is_unsafe(*character),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use unicode_bidi::format_chars::{ALM, FSI, LRE, LRI, LRM, LRO, PDF, PDI, RLE, RLI, RLM, RLO};

    use super::{Keep, LINE_SEPARATOR, PARAGRAPH_SEPARATOR, is_unsafe, strip_unsafe};

    const BIDI: [char; 12] = [ALM, LRM, RLM, LRE, RLE, PDF, LRO, RLO, LRI, RLI, FSI, PDI];

    /// C0, DEL, and C1, as UnicodeData.txt lists general category Cc.
    fn controls() -> impl Iterator<Item = char> {
        ('\u{0}'..='\u{1f}').chain('\u{7f}'..='\u{9f}')
    }

    #[test]
    fn refuses_every_control() {
        assert_eq!(controls().count(), 65);
        for character in controls() {
            assert!(is_unsafe(character), "{character:?} passed");
        }
    }

    #[test]
    fn refuses_every_bidi_formatting_character() {
        for character in BIDI {
            assert!(is_unsafe(character), "{character:?} passed");
        }
    }

    #[test]
    fn refuses_both_separators() {
        assert!(is_unsafe(LINE_SEPARATOR));
        assert!(is_unsafe(PARAGRAPH_SEPARATOR));
    }

    #[test]
    fn keeps_the_neighbours_of_each_set() {
        let neighbours = [
            ' ', '~', '\u{a0}', '\u{61b}', '\u{61d}', '\u{200d}', '\u{2010}', '\u{2027}',
            '\u{202f}', '\u{2065}', '\u{206a}',
        ];
        for character in neighbours {
            assert!(!is_unsafe(character), "{character:?} refused");
        }
    }

    #[test]
    fn keeps_the_other_format_characters() {
        let kept = [
            '\u{ad}',
            '\u{600}',
            '\u{200b}',
            '\u{200c}',
            '\u{200d}',
            '\u{2060}',
            '\u{feff}',
            '\u{e0001}',
            '\u{e0067}',
            '\u{e007f}',
        ];
        for character in kept {
            assert!(!is_unsafe(character), "{character:?} refused");
        }
    }

    #[test]
    fn refuses_nothing_outside_the_three_sets() {
        let refused = (0..=u32::from(char::MAX))
            .filter_map(char::from_u32)
            .filter(|character| is_unsafe(*character))
            .count();
        assert_eq!(refused, 65 + BIDI.len() + 2);
    }

    #[test]
    fn strips_every_unsafe_character_and_keeps_the_rest() {
        let text = "a\u{1b}[2J b\u{202e}c\u{2028}d\u{7f}\tx\ny";
        assert_eq!(strip_unsafe(text, Keep::Nothing), "a[2J bcdxy");
    }

    #[test]
    fn keeps_newline_and_tab_when_asked() {
        let text = "a\u{1b}b\tc\nd\u{2029}e";
        assert_eq!(strip_unsafe(text, Keep::NewlineAndTab), "ab\tc\nde");
    }

    #[test]
    fn strips_the_whole_unsafe_set_and_nothing_else() {
        let everything: String = (0..=u32::from(char::MAX))
            .filter_map(char::from_u32)
            .collect();
        let stripped = strip_unsafe(&everything, Keep::Nothing);
        assert_eq!(
            everything.chars().count() - stripped.chars().count(),
            65 + BIDI.len() + 2
        );
        assert!(stripped.chars().all(|character| !is_unsafe(character)));
    }

    #[test]
    fn unicode_bidi_still_targets_the_documented_version() {
        assert_eq!(unicode_bidi::UNICODE_VERSION, (16, 0, 0));
    }
}

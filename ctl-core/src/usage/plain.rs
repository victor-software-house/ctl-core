//! clap's help text without its terminal control sequences.
//!
//! clap keeps help text as written, escape sequences included. Its `Display`
//! strips them only when clap's `color` feature is on. ctl-core does not turn
//! that feature on, but another crate in a consumer's graph may. [`plain`]
//! removes what clap removes with `color` on, so the spec holds the same bytes
//! either way. Over text clap already stripped, it changes nothing.
//!
//! clap strips through anstream 1.0.0's [`strip_str`]. That runs the
//! [anstyle-parse 1.0.0 state machine][states], which reads [ECMA-48] control
//! sequences as Paul Williams' [DEC parser][dec] does. [`strip`] follows it
//! byte for byte:
//!
//! - In the ground state, it drops ESC, which starts a sequence, and DEL. It
//!   drops each C0 control but tab, line feed, form feed, and carriage return,
//!   and keeps every other character.
//! - Inside a sequence, it keeps nothing. Inside an escape or CSI sequence,
//!   tab, line feed, form feed, and carriage return are kept, and end it.
//! - CAN and SUB end any sequence, and ESC starts a new one.
//! - An escape sequence and a CSI end at their final byte, and an OSC at BEL. A
//!   DCS, SOS, PM, or APC ends at byte 0x9C.
//! - Byte 0x9C is also a UTF-8 continuation byte. So a character such as `“`
//!   (U+201C) ends a DCS, SOS, PM, or APC, and the rest of a character cut that
//!   way is dropped.
//!
//! [`strip_str`]: https://docs.rs/anstream/1.0.0/anstream/adapter/fn.strip_str.html
//! [states]: https://docs.rs/crate/anstyle-parse/1.0.0/source/src/state/codegen.rs
//! [dec]: https://vt100.net/emu/dec_ansi_parser
//! [ECMA-48]: https://ecma-international.org/publications-and-standards/standards/ecma-48/

use clap::builder::StyledStr;

/// The text of `styled` with its terminal control sequences removed, the same
/// whether or not clap's `color` feature is on.
pub(crate) fn plain(styled: &StyledStr) -> String {
    strip(&styled.to_string())
}

/// `text` with what anstream 1.0.0's `strip_str` removes taken out.
fn strip(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut sequence: Option<Sequence> = None;
    for c in text.chars() {
        let Some(mut current) = sequence else {
            match c {
                '\u{1b}' => sequence = Some(Sequence::Escape),
                '\t' | '\n' | '\u{c}' | '\r' => plain.push(c),
                c if c.is_ascii_control() => {}
                c => plain.push(c),
            }
            continue;
        };
        let mut buffer = [0; 4];
        for &byte in c.encode_utf8(&mut buffer).as_bytes() {
            match current.after(byte) {
                Next::Stay => {}
                Next::Enter(next) => current = next,
                Next::Print => {
                    plain.push(c);
                    sequence = None;
                    break;
                }
                Next::Leave => {
                    sequence = None;
                    break;
                }
            }
            sequence = Some(current);
        }
    }
    plain
}

/// A control sequence being read, by the state anstyle-parse is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sequence {
    /// After ESC.
    Escape,
    /// After ESC and an intermediate byte.
    EscapeIntermediate,
    /// A CSI, in any of its four states: all of them end at the same bytes.
    Csi,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    /// The data of a DCS, or the rest of one anstyle-parse ignores.
    DcsData,
    /// An SOS, PM, or APC string.
    Data,
    Osc,
}

/// What one byte does to the sequence being read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Next {
    Stay,
    Enter(Sequence),
    /// The byte ends the sequence and is dropped.
    Leave,
    /// The byte ends the sequence and is kept: whitespace anstyle-parse
    /// executes inside an escape or CSI sequence.
    Print,
}

impl Sequence {
    fn after(self, byte: u8) -> Next {
        match byte {
            0x18 | 0x1a => return Next::Leave,
            0x1b => return Next::Enter(Self::Escape),
            _ => {}
        }
        match self {
            Self::Escape => match byte {
                0x00..=0x1f => execute(byte),
                0x20..=0x2f => Next::Enter(Self::EscapeIntermediate),
                b'[' => Next::Enter(Self::Csi),
                b']' => Next::Enter(Self::Osc),
                b'P' => Next::Enter(Self::DcsEntry),
                b'X' | b'^' | b'_' => Next::Enter(Self::Data),
                0x30..=0x7e => Next::Leave,
                _ => Next::Stay,
            },
            Self::EscapeIntermediate => match byte {
                0x00..=0x1f => execute(byte),
                0x30..=0x7e => Next::Leave,
                _ => Next::Stay,
            },
            Self::Csi => match byte {
                0x00..=0x1f => execute(byte),
                0x40..=0x7e => Next::Leave,
                _ => Next::Stay,
            },
            Self::DcsEntry => match byte {
                0x20..=0x2f => Next::Enter(Self::DcsIntermediate),
                0x30..=0x3f => Next::Enter(Self::DcsParam),
                0x40..=0x7e => Next::Enter(Self::DcsData),
                _ => Next::Stay,
            },
            Self::DcsParam => match byte {
                0x20..=0x2f => Next::Enter(Self::DcsIntermediate),
                0x3c..=0x7e => Next::Enter(Self::DcsData),
                _ => Next::Stay,
            },
            Self::DcsIntermediate => match byte {
                0x30..=0x7e => Next::Enter(Self::DcsData),
                _ => Next::Stay,
            },
            Self::DcsData | Self::Data => match byte {
                0x9c => Next::Leave,
                _ => Next::Stay,
            },
            Self::Osc => match byte {
                0x07 => Next::Leave,
                _ => Next::Stay,
            },
        }
    }
}

/// A C0 control inside an escape or CSI sequence: anstyle-parse executes it,
/// and anstream keeps it only when it is whitespace.
fn execute(byte: u8) -> Next {
    match byte {
        b'\t' | b'\n' | 0x0c | b'\r' => Next::Print,
        _ => Next::Stay,
    }
}

#[cfg(test)]
mod tests {
    use super::strip;

    /// Each input with what anstream 1.0.0 leaves of it, worked out from the
    /// anstyle-parse 1.0.0 state table.
    const CASES: &[(&str, &str)] = &[
        ("a\u{1b}[1mb", "ab"),
        ("a\u{7f}b", "ab"),
        ("a\u{7}\u{b}\tb\r\n\u{c}", "a\tb\r\n\u{c}"),
        ("café \u{85}\u{2028}", "café \u{85}\u{2028}"),
        ("\u{1b}]8;;id\u{7}link\u{1b}]8;;\u{1b}\\", "link"),
        ("\u{1b}]é\u{7}x", "x"),
        ("\u{1b}[1\nm", "\nm"),
        ("\u{1b}[1\u{18}m", "m"),
        ("\u{1b}(Bx", "x"),
        ("\u{1b}Pq#0\u{1b}\\x", "x"),
        ("\u{1b}P\u{9c}x", ""),
        ("\u{1b}P1;2|\u{201c}x", "x"),
        ("\u{1b}_a\u{201c}b", "b"),
        ("\u{1b}_\u{2713}b", "b"),
        ("\u{1b}^\nb\u{1b}\\c", "c"),
        ("a\u{1b}", "a"),
    ];

    #[test]
    fn strips_what_anstream_strips() {
        for (input, expected) in CASES {
            assert_eq!(strip(input), *expected, "{input:?}");
        }
    }

    #[test]
    fn stripped_text_strips_to_itself() {
        for (input, _) in CASES {
            let once = strip(input);
            assert_eq!(strip(&once), once, "{input:?}");
        }
    }
}

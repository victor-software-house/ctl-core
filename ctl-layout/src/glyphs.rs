//! The characters a frame is drawn with, for Unicode and for ASCII.

use ctl_text::{BoxChars, Line, Span, Style};

/// The four characters of one horizontal line of a grid. They are its left
/// end, the character repeated across a column, the junction under a column
/// rule, and its right end.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Border {
    left: char,
    fill: char,
    junction: char,
    right: char,
}

impl Border {
    const fn new(left: char, fill: char, junction: char, right: char) -> Self {
        Self {
            left,
            fill,
            junction,
            right,
        }
    }

    /// The line across columns of the given content widths, each with one
    /// cell of padding on both sides.
    pub(crate) fn line(self, widths: &[u16]) -> Line {
        let mut text = String::from(self.left);
        for (index, width) in widths.iter().enumerate() {
            if index > 0 {
                text.push(self.junction);
            }
            text.extend(std::iter::repeat_n(self.fill, usize::from(*width) + 2));
        }
        text.push(self.right);
        Line::from(Span::new(text, Style::new()))
    }
}

/// Every character a table, record, or rule draws.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Glyphs {
    pub(crate) top: Border,
    pub(crate) header: Border,
    pub(crate) row: Border,
    pub(crate) bottom: Border,
    pub(crate) outer: char,
    pub(crate) inner: char,
    pub(crate) rule: char,
}

const UNICODE: Glyphs = Glyphs {
    top: Border::new('┌', '─', '┬', '┐'),
    header: Border::new('╞', '═', '╪', '╡'),
    row: Border::new('├', '╌', '┼', '┤'),
    bottom: Border::new('└', '─', '┴', '┘'),
    outer: '│',
    inner: '┆',
    rule: '─',
};

const ASCII: Glyphs = Glyphs {
    top: Border::new('+', '-', '+', '+'),
    header: Border::new('+', '=', '+', '+'),
    row: Border::new('+', '-', '+', '+'),
    bottom: Border::new('+', '-', '+', '+'),
    outer: '|',
    inner: '|',
    rule: '-',
};

impl Glyphs {
    /// The set for `box_chars`. A set added later falls back to Unicode.
    pub(crate) fn for_chars(box_chars: BoxChars) -> Self {
        if box_chars == BoxChars::Ascii {
            ASCII
        } else {
            UNICODE
        }
    }
}

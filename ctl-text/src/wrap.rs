//! Fitting a line to a width: wrapping and truncation.

use anstyle::Style;

use crate::cells::{prefix_len, text_width};
use crate::line::{Line, Piece};

/// The text an [`Overflow::Ellipsis`] cut ends with. One cell wide.
pub const ELLIPSIS: &str = "…";

/// What happens to a line wider than the width it is given.
///
/// Each variant is a policy a layout offers a column or a paragraph. No
/// policy splits a grapheme cluster.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Overflow {
    /// Break lines at spaces only. A word wider than the width keeps a line
    /// of its own and runs past the width.
    #[default]
    KeepWords,
    /// Break lines at spaces, and break a word wider than the width between
    /// clusters. A cluster wider than the width keeps a line of its own.
    Fold,
    /// Keep one line and cut it with an [`ELLIPSIS`] when it is too wide.
    Ellipsis,
}

impl Line {
    /// The width of the widest part of the line that `overflow` will not
    /// break: the minimum width it can be laid out in.
    pub(crate) fn minimum_width(&self, overflow: Overflow) -> u16 {
        let pieces = self.pieces();
        match overflow {
            Overflow::KeepWords => pieces
                .split(|piece| piece.space)
                .map(total_width)
                .max()
                .unwrap_or(0),
            Overflow::Fold => pieces
                .iter()
                .filter(|piece| !piece.space)
                .map(|piece| piece.width)
                .max()
                .unwrap_or(0),
            Overflow::Ellipsis => u16::from(self.width() > 0),
        }
    }

    /// The line cut to exactly `width` cells and ended with [`ELLIPSIS`], or
    /// the line itself when it fits.
    ///
    /// The text is cut at a cluster boundary. A wide cluster that straddles
    /// the cut is replaced by a space, so a cut line is never narrower than
    /// `width`. The padding and the ellipsis take the style of the text they
    /// replace. A width of 0 gives an empty line.
    #[must_use]
    pub fn truncate(&self, width: u16) -> Self {
        if self.width() <= width {
            return self.clone();
        }
        let Some(budget) = width.checked_sub(ELLIPSIS_WIDTH) else {
            return Self::new();
        };
        let mut cut = Self::new();
        let mut used: u16 = 0;
        let mut style = Style::new();
        for span in self.spans() {
            style = span.style();
            let span_width = span.width();
            if used.saturating_add(span_width) <= budget {
                cut.push(span.clone());
                used = used.saturating_add(span_width);
            } else {
                let room = budget.saturating_sub(used);
                let kept = &span.text()[..prefix_len(span.text(), room)];
                let gap = usize::from(room.saturating_sub(text_width(kept)));
                cut.append(kept, style);
                cut.append(&" ".repeat(gap), style);
                break;
            }
        }
        cut.append(ELLIPSIS, style);
        cut
    }

    /// The line fitted to `width` cells by `overflow`: wrapped into lines for
    /// [`Overflow::KeepWords`] and [`Overflow::Fold`], or cut into one line
    /// for [`Overflow::Ellipsis`].
    ///
    /// A wrapped line never ends in a space: the spaces at the end of the line
    /// and at each break are dropped. An ellipsis line that fits keeps the
    /// spaces it ends with, as [`Line::truncate`] does. Call
    /// [`trim_end`](Line::trim_end) first; [`Text`](crate::Text) does this for
    /// you. A width of 0 counts as 1 when wrapping.
    #[must_use]
    pub fn fit(&self, width: u16, overflow: Overflow) -> Vec<Self> {
        match overflow {
            Overflow::KeepWords | Overflow::Fold => self.wrap(width, overflow),
            Overflow::Ellipsis => vec![self.truncate(width)],
        }
    }

    /// The line broken into lines no wider than `width`, except where
    /// `overflow` lets a word or a cluster run past it.
    fn wrap(&self, width: u16, overflow: Overflow) -> Vec<Self> {
        let pieces = self.pieces();
        let mut wrapper = Wrapper::new(width.max(1), overflow);
        let mut rest = pieces.as_slice();
        while !rest.is_empty() {
            let space_len = rest.iter().take_while(|piece| piece.space).count();
            let (space, tail) = rest.split_at(space_len);
            let word_len = tail.iter().take_while(|piece| !piece.space).count();
            let (word, next) = tail.split_at(word_len);
            if word.is_empty() {
                break;
            }
            wrapper.place(space, word);
            rest = next;
        }
        wrapper.finish()
    }
}

const ELLIPSIS_WIDTH: u16 = 1;

fn total_width(pieces: &[Piece<'_>]) -> u16 {
    pieces
        .iter()
        .fold(0, |total, piece| total.saturating_add(piece.width))
}

/// The line being built and the lines already done.
///
/// A word is placed after the spaces that came before it. Leading spaces
/// stay only on the first line, as indentation, and only when the word still
/// fits behind them. Every other space at a break is dropped, so no line
/// ends or starts with the spaces of a break.
struct Wrapper<'a> {
    width: u16,
    overflow: Overflow,
    lines: Vec<Line>,
    current: Vec<Piece<'a>>,
    current_width: u16,
}

impl<'a> Wrapper<'a> {
    fn new(width: u16, overflow: Overflow) -> Self {
        Self {
            width,
            overflow,
            lines: Vec::new(),
            current: Vec::new(),
            current_width: 0,
        }
    }

    fn place(&mut self, space: &[Piece<'a>], word: &[Piece<'a>]) {
        let space_width = total_width(space);
        let word_width = total_width(word);
        if self.current.is_empty() {
            let indented =
                self.lines.is_empty() && space_width.saturating_add(word_width) <= self.width;
            if indented {
                self.extend(space);
            }
            self.put_word(word, word_width);
        } else if self
            .current_width
            .saturating_add(space_width)
            .saturating_add(word_width)
            <= self.width
        {
            self.extend(space);
            self.extend(word);
        } else {
            self.break_line();
            self.put_word(word, word_width);
        }
    }

    /// Adds a word to the empty or fitting current line, folding it between
    /// clusters when the policy allows and it is wider than a line.
    fn put_word(&mut self, word: &[Piece<'a>], word_width: u16) {
        if self.overflow == Overflow::Fold && word_width > self.width {
            for piece in word {
                let overflows = self.current_width.saturating_add(piece.width) > self.width;
                if overflows && !self.current.is_empty() {
                    self.break_line();
                }
                self.push(*piece);
            }
        } else {
            self.extend(word);
        }
    }

    fn extend(&mut self, pieces: &[Piece<'a>]) {
        for piece in pieces {
            self.push(*piece);
        }
    }

    fn push(&mut self, piece: Piece<'a>) {
        self.current_width = self.current_width.saturating_add(piece.width);
        self.current.push(piece);
    }

    fn break_line(&mut self) {
        self.lines.push(Line::from_pieces(&self.current));
        self.current.clear();
        self.current_width = 0;
    }

    fn finish(mut self) -> Vec<Line> {
        if !self.current.is_empty() || self.lines.is_empty() {
            self.break_line();
        }
        self.lines
    }
}

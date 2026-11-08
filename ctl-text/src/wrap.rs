//! Fitting a line to a width: wrapping and truncation.

use anstyle::Style;

use crate::cells::prefix_len;
use crate::line::{Line, Piece};

/// The text an [`Overflow::Ellipsis`] cut ends with. One cell wide.
pub const ELLIPSIS: &str = "…";

/// What to do with a line wider than its width.
///
/// The variants are the three policies a layout offers a column or a
/// paragraph. A policy never splits a grapheme cluster.
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

/// The width of the widest part of `line` that `overflow` will not break.
///
/// This is the minimum width the line can be laid out in.
pub(crate) fn minimum(line: &Line, overflow: Overflow) -> u16 {
    let pieces = line.pieces();
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
        Overflow::Ellipsis => u16::from(line.width() > 0),
    }
}

/// `line` cut to `width` cells, ended with [`ELLIPSIS`]; see
/// [`Line::truncate`].
pub(crate) fn truncate(line: &Line, width: u16) -> Line {
    if line.width() <= width {
        return line.clone();
    }
    let Some(budget) = width.checked_sub(ELLIPSIS_WIDTH) else {
        return Line::new();
    };
    let mut cut = Line::new();
    let mut used: u16 = 0;
    let mut style = Style::new();
    for span in line.spans() {
        style = span.style();
        let span_width = span.width();
        if used.saturating_add(span_width) <= budget {
            cut.push(span.clone());
            used = used.saturating_add(span_width);
        } else {
            let length = prefix_len(span.text(), budget.saturating_sub(used));
            cut.append(&span.text()[..length], style);
            break;
        }
    }
    cut.append(ELLIPSIS, style);
    cut
}

const ELLIPSIS_WIDTH: u16 = 1;

/// `line` broken into lines no wider than `width`, except where `overflow`
/// lets a word or a cluster run past it; see [`Line::fit`].
pub(crate) fn wrap(line: &Line, width: u16, overflow: Overflow) -> Vec<Line> {
    let pieces = line.pieces();
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

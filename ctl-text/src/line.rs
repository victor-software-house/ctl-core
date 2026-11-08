//! One line of styled spans.

use std::fmt;

use anstyle::Style;
use unicode_segmentation::UnicodeSegmentation;

use crate::cells::{cluster_width, is_space};
use crate::span::Span;
use crate::wrap::{self, Overflow};

/// One grapheme cluster with the style of its span.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Piece<'a> {
    pub(crate) text: &'a str,
    pub(crate) style: Style,
    pub(crate) width: u16,
    pub(crate) space: bool,
}

/// A list of spans that sit on one terminal line.
///
/// The text of a line is the text of its spans, joined. Its [`Display`]
/// prints that text without styles.
///
/// [`Display`]: fmt::Display
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Line {
    spans: Vec<Span>,
}

impl Line {
    /// An empty line.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `span` at the end of the line.
    ///
    /// When `span` starts with characters that continue the line's last
    /// grapheme cluster, such as a combining mark or the rest of a ZWJ emoji
    /// sequence, they join the previous span and take its style, since a
    /// terminal cannot style part of a cluster. So no cluster crosses a span
    /// boundary and every width counts the cluster once. An empty span adds
    /// nothing, so none can sit between a cluster's parts.
    pub fn push(&mut self, span: Span) {
        if span.text.is_empty() {
            return;
        }
        self.spans.push(span);
        self.keep_clusters_whole();
    }

    /// The spans, in order.
    #[must_use]
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// The width of the line in terminal cells. A width over `u16::MAX`
    /// saturates.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.spans
            .iter()
            .map(Span::width)
            .fold(0, u16::saturating_add)
    }

    /// Whether the line holds no text.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(|span| span.text.is_empty())
    }

    /// Removes the spaces at the end of the line, and any span they empty.
    pub fn trim_end(&mut self) {
        while let Some(last) = self.spans.last_mut() {
            let trailing = trailing_space_len(&last.text);
            if trailing == last.text.len() {
                self.spans.pop();
            } else {
                let kept = last.text.len() - trailing;
                last.text.truncate(kept);
                break;
            }
        }
    }

    /// The line cut to at most `width` cells and ended with
    /// [`ELLIPSIS`](crate::ELLIPSIS), or the line itself when it fits.
    ///
    /// The text is cut at a cluster boundary, so a wide cluster that does not
    /// fit is dropped whole and the result can be one cell narrower than
    /// `width`. The ellipsis takes the style of the text it replaces. A width
    /// of 0 gives an empty line.
    #[must_use]
    pub fn truncate(&self, width: u16) -> Self {
        wrap::truncate(self, width)
    }

    /// The line fitted to `width` cells by `overflow`: wrapped into lines for
    /// [`Overflow::KeepWords`] and [`Overflow::Fold`], or cut into one line
    /// for [`Overflow::Ellipsis`].
    ///
    /// No resulting line ends in a space: the spaces at the end of the line
    /// and at each break are dropped. A width of 0 counts as 1 when wrapping.
    #[must_use]
    pub fn fit(&self, width: u16, overflow: Overflow) -> Vec<Self> {
        let mut trimmed = self.clone();
        trimmed.trim_end();
        match overflow {
            Overflow::KeepWords | Overflow::Fold => wrap::wrap(&trimmed, width, overflow),
            Overflow::Ellipsis => vec![wrap::truncate(&trimmed, width)],
        }
    }

    /// Adds `text` in `style`, joining the last span when it has the same
    /// style. `text` must already be safe.
    pub(crate) fn append(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        match self.spans.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            Some(_) | None => {
                self.spans.push(Span {
                    text: text.to_owned(),
                    style,
                });
                self.keep_clusters_whole();
            }
        }
    }

    /// Moves the start of the last span into the span before it when that
    /// start continues the earlier span's last cluster. The cluster keeps the
    /// style it started in.
    fn keep_clusters_whole(&mut self) {
        let [.., before, last] = self.spans.as_mut_slice() else {
            return;
        };
        let Some(tail) = before.text.graphemes(true).next_back() else {
            return;
        };
        let tail_len = tail.len();
        let joined = format!("{tail}{}", last.text);
        let Some(first) = joined.graphemes(true).next() else {
            return;
        };
        let moved = first.len().saturating_sub(tail_len);
        if moved == 0 {
            return;
        }
        let carried: String = last.text.drain(..moved).collect();
        before.text.push_str(&carried);
        if last.text.is_empty() {
            self.spans.pop();
        }
    }

    pub(crate) fn pieces(&self) -> Vec<Piece<'_>> {
        self.spans
            .iter()
            .flat_map(|span| {
                span.text.graphemes(true).map(|cluster| Piece {
                    text: cluster,
                    style: span.style,
                    width: cluster_width(cluster),
                    space: is_space(cluster),
                })
            })
            .collect()
    }

    pub(crate) fn from_pieces(pieces: &[Piece<'_>]) -> Self {
        let mut line = Self::new();
        for piece in pieces {
            line.append(piece.text, piece.style);
        }
        line
    }
}

/// The byte length of the clusters at the end of `text` that are all spaces.
fn trailing_space_len(text: &str) -> usize {
    text.graphemes(true)
        .rev()
        .take_while(|cluster| is_space(cluster))
        .map(str::len)
        .sum()
}

impl From<Span> for Line {
    fn from(span: Span) -> Self {
        let mut line = Self::new();
        line.push(span);
        line
    }
}

impl FromIterator<Span> for Line {
    fn from_iter<I: IntoIterator<Item = Span>>(spans: I) -> Self {
        let mut line = Self::new();
        for span in spans {
            line.push(span);
        }
        line
    }
}

impl fmt::Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.spans
            .iter()
            .try_for_each(|span| formatter.write_str(&span.text))
    }
}

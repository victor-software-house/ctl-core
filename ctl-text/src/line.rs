//! One line of styled spans.

use std::fmt;

use anstyle::Style;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete, UnicodeSegmentation};

use crate::cells::{cluster_width, is_space};
use crate::span::Span;

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
/// prints that text without styles, which suits tests and plain output.
/// Build one with `Line::from(span)`, or collect spans into it.
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
    /// A span in the style of the line's last span joins it, so two lines
    /// with the same text and styles are equal however their spans were
    /// pushed.
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
        match self.spans.last_mut() {
            Some(last) if last.style == span.style => last.text.push_str(&span.text),
            Some(_) | None => self.push_new(span),
        }
    }

    /// The spans, in order. Their text joined is the text of the line.
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
    ///
    /// A no-break space (U+00A0, U+2007, or U+202F) is not a space here, so it
    /// stays.
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

    /// Adds `text` in `style`, joining the last span when it has the same
    /// style. `text` must already be safe.
    pub(crate) fn append(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        match self.spans.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            Some(_) | None => self.push_new(Span {
                text: text.to_owned(),
                style,
            }),
        }
    }

    /// Adds a non-empty `span` whose style differs from the last span's.
    fn push_new(&mut self, span: Span) {
        self.spans.push(span);
        self.keep_clusters_whole();
    }

    /// Moves the start of the last span into the span before it when that
    /// start continues the earlier span's last cluster. The cluster keeps the
    /// style it started in.
    fn keep_clusters_whole(&mut self) {
        let [.., before, last] = self.spans.as_mut_slice() else {
            return;
        };
        let moved = continuation_len(&before.text, &last.text);
        if moved == 0 {
            return;
        }
        before.text.extend(last.text.drain(..moved));
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

/// The byte length of the start of `after` that continues the last grapheme
/// cluster of `before`, or 0 when `after` starts a new cluster.
///
/// A cursor over `before` followed by `after` decides the boundary at their
/// join, so no joined copy of the text is built.
fn continuation_len(before: &str, after: &str) -> usize {
    let junction = before.len();
    let mut cursor = GraphemeCursor::new(junction, junction + after.len(), true);
    loop {
        match cursor.is_boundary(after, junction) {
            Ok(true)
            | Err(
                GraphemeIncomplete::PrevChunk
                | GraphemeIncomplete::NextChunk
                | GraphemeIncomplete::InvalidOffset,
            ) => return 0,
            Ok(false) => break,
            Err(GraphemeIncomplete::PreContext(end)) => cursor.provide_context(&before[..end], 0),
        }
    }
    loop {
        match cursor.next_boundary(after, junction) {
            Ok(Some(end)) => return end.saturating_sub(junction),
            Ok(None) => return after.len(),
            Err(GraphemeIncomplete::PreContext(end)) => cursor.provide_context(&before[..end], 0),
            Err(
                GraphemeIncomplete::PrevChunk
                | GraphemeIncomplete::NextChunk
                | GraphemeIncomplete::InvalidOffset,
            ) => return 0,
        }
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

impl Extend<Span> for Line {
    fn extend<I: IntoIterator<Item = Span>>(&mut self, spans: I) {
        for span in spans {
            self.push(span);
        }
    }
}

impl FromIterator<Span> for Line {
    fn from_iter<I: IntoIterator<Item = Span>>(spans: I) -> Self {
        let mut line = Self::new();
        line.extend(spans);
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

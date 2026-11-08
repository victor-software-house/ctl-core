//! Entries drawn as labels over an indented description.

use std::num::NonZeroU16;

use ctl_text::{Context, Line, Measurement, Overflow, Renderable, Text};

use crate::lines::{indent_lines, literal};

/// The indent of an entry's label line.
const LABEL_INDENT: u16 = 2;
/// The indent of an entry's description lines.
const DESCRIPTION_INDENT: u16 = 4;

/// Entries, each a line of labels over indented lines of description:
///
/// ```text
///   -f --format
///     Output representation
/// ```
///
/// It is the form of a table of flags or commands for a narrow width, where
/// the columns no longer fit side by side. Put it in a
/// [`Breakpoint`](crate::Breakpoint) beside the table. The labels of an entry
/// share one line, separated by a space, and its description cells share the
/// lines below, wrapped at spaces to the width that is left. An entry whose
/// labels or description are empty skips that part.
#[derive(Clone, Debug, Default)]
pub struct Definitions {
    entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
struct Entry {
    labels: Vec<Text>,
    description: Vec<Text>,
}

impl Definitions {
    /// A list with no entries.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entry: its `labels` on a line indented by 2, and its
    /// `description` below them, indented by 4. Each cell converts to a
    /// [`Text`], and a cell with no text adds nothing.
    #[must_use]
    pub fn entry<L, D>(
        self,
        labels: impl IntoIterator<Item = L>,
        description: impl IntoIterator<Item = D>,
    ) -> Self
    where
        L: Into<Text>,
        D: Into<Text>,
    {
        self.push(
            labels.into_iter().map(Into::into).collect(),
            description.into_iter().map(Into::into).collect(),
        )
    }

    fn push(mut self, labels: Vec<Text>, description: Vec<Text>) -> Self {
        self.entries.push(Entry {
            labels,
            description,
        });
        self
    }

    /// Every entry laid out at `width`.
    fn lines(&self, width: u16, tab_width: NonZeroU16) -> Vec<Line> {
        let mut lines = Vec::new();
        for entry in &self.entries {
            let labels = join_cells(&entry.labels, tab_width);
            let description = join_cells(&entry.description, tab_width);
            for (part, indent) in [(labels, LABEL_INDENT), (description, DESCRIPTION_INDENT)] {
                if !part.is_empty() {
                    let fitted = part.fit(width.saturating_sub(indent), Overflow::KeepWords);
                    lines.extend(indent_lines(fitted, indent));
                }
            }
        }
        lines
    }
}

/// The non-empty lines of `cells` on one line, separated by a space.
fn join_cells(cells: &[Text], tab_width: NonZeroU16) -> Line {
    let mut joined = Line::new();
    for line in cells
        .iter()
        .flat_map(|cell| cell.lines(tab_width))
        .filter(|line| !line.is_empty())
    {
        if !joined.is_empty() {
            joined.push(literal(" "));
        }
        joined.extend(line.spans().iter().cloned());
    }
    joined
}

impl Renderable for Definitions {
    fn measure(&self, context: &Context) -> Measurement {
        let widest = |width| {
            self.lines(width, context.tab_width())
                .iter()
                .map(Line::width)
                .max()
                .unwrap_or(0)
        };
        Measurement::new(widest(1), widest(u16::MAX))
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.lines(context.width(), context.tab_width())
    }
}

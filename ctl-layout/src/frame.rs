//! How a table or a record is framed.

use crate::widths::Framing;

/// What a table draws around and between its columns.
///
/// Rules are drawn with box characters, or with ASCII for
/// [`BoxChars::Ascii`](ctl_text::BoxChars::Ascii). The frame decides how a
/// table treats a word wider than its column: [`Grid`](Self::Grid) folds it,
/// and the other two let it run past the width.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Frame {
    /// An outer frame, column rules, and a rule under the header. A word
    /// wider than its column folds, so the frame stays aligned.
    #[default]
    Grid,
    /// One rule under the header and nothing else. A table with no header
    /// draws no rule.
    HeaderRule,
    /// No rules at all. Columns are separated by two spaces, and no line
    /// ends in a space.
    Plain,
}

impl Frame {
    /// Whether the frame has an outer border that a wide word would break.
    /// One match here keeps every width decision of a table in step.
    pub(crate) fn framing(self) -> Framing {
        match self {
            Self::Grid => Framing::Framed,
            Self::HeaderRule | Self::Plain => Framing::Unframed,
        }
    }
}

/// What separates consecutive rows of a table.
///
/// The separator goes between rows only. It is never drawn above the first
/// row, below the last, or between the header and the first row.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RowSeparation {
    /// Rows print on consecutive lines.
    #[default]
    None,
    /// A horizontal rule between each pair of rows, as wide as the table.
    Rule,
    /// A blank row between each pair of rows. In a grid it keeps the column
    /// rules.
    Blank,
}

/// How a [`Record`](crate::Record) of key and value rows is drawn.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RecordStyle {
    /// No frame. Keys are right-aligned in one column.
    #[default]
    KeysRight,
    /// No frame. Keys are left-aligned in one column.
    KeysLeft,
    /// An outer frame with one rule between the keys and the values. Keys are
    /// left-aligned.
    Boxed,
}

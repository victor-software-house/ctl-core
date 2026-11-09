//! How a table or a record is framed.

use crate::widths::Framing;

/// What a table draws around and between its columns.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Frame {
    /// An outer frame, column rules, and a rule under the header. A word
    /// wider than its column folds, so the frame stays aligned.
    #[default]
    Grid,
    /// One rule under the header and nothing else.
    HeaderRule,
    /// No rules at all. Columns are separated by two spaces.
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
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RowSeparation {
    /// Rows print on consecutive lines.
    #[default]
    None,
    /// A horizontal rule between each pair of rows.
    Rule,
    /// A blank row between each pair of rows.
    Blank,
}

/// How a record of key and value rows is drawn.
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

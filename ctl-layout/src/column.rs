//! One column of a table.

use ctl_text::{Overflow, Text};

use crate::align::Align;

/// A column of a table: its header, what it does with a cell wider than it
/// gets, and which edge its cells sit against.
#[derive(Clone, Debug)]
pub struct Column {
    pub(crate) header: Option<Text>,
    pub(crate) overflow: Overflow,
    pub(crate) align: Align,
}

impl Column {
    /// A column headed by `header`, in the default style.
    #[must_use]
    pub fn new(header: impl AsRef<str>) -> Self {
        Self::headed(Some(Text::plain(header)))
    }

    /// A column headed by an already styled `header`.
    #[must_use]
    pub fn styled(header: Text) -> Self {
        Self::headed(Some(header))
    }

    /// A column with no header. A table draws a header row only when at least
    /// one of its columns has a header.
    #[must_use]
    pub fn untitled() -> Self {
        Self::headed(None)
    }

    fn headed(header: Option<Text>) -> Self {
        Self {
            header,
            overflow: Overflow::default(),
            align: Align::default(),
        }
    }

    /// What the column does with a cell wider than it gets. The default is
    /// [`Overflow::KeepWords`].
    #[must_use]
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Aligns every cell of the column to the right, header included, so
    /// numbers line up on their last digit.
    #[must_use]
    pub fn numeric(self) -> Self {
        self.align(Align::Right)
    }

    /// Aligns every cell of the column, header included, against `align`.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }
}

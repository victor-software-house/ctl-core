//! The protocol a widget follows: measure, then render.

use crate::cells::TAB_WIDTH;
use crate::line::Line;

/// The narrowest and widest a piece of content can usefully be, in cells.
///
/// The minimum is the widest unit the content will not break: a word under
/// [`Overflow::KeepWords`](crate::Overflow::KeepWords), a cluster under
/// [`Overflow::Fold`](crate::Overflow::Fold). The maximum is the width the
/// content takes when nothing wraps it. The maximum is never below the
/// minimum.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Measurement {
    minimum: u16,
    maximum: u16,
}

impl Measurement {
    /// A measurement from `minimum` to `maximum`. A `maximum` below
    /// `minimum` is raised to it.
    #[must_use]
    pub fn new(minimum: u16, maximum: u16) -> Self {
        Self {
            minimum,
            maximum: maximum.max(minimum),
        }
    }

    /// A measurement of content that is always `width` cells wide.
    #[must_use]
    pub fn exact(width: u16) -> Self {
        Self::new(width, width)
    }

    /// The narrowest width the content can be laid out in.
    #[must_use]
    pub fn minimum(self) -> u16 {
        self.minimum
    }

    /// The width the content takes when nothing wraps it.
    #[must_use]
    pub fn maximum(self) -> u16 {
        self.maximum
    }
}

/// The characters a widget draws borders and rules with.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BoxChars {
    /// Unicode box-drawing characters.
    #[default]
    Unicode,
    /// ASCII only: `+`, `-`, and `|`.
    Ascii,
}

/// What a widget is told when it measures and renders: the width to fill, the
/// tab stop spacing, and the box characters.
///
/// A context holds no colour or terminal fact. Styles are resolved before
/// layout, and escape sequences are written after it.
///
/// The `with_` methods return a changed copy, so a container derives the
/// context of a child from its own: `context.with_width(column)`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Context {
    width: u16,
    tab_width: u16,
    box_chars: BoxChars,
}

impl Context {
    /// A context `width` cells wide, with tab stops every [`TAB_WIDTH`]
    /// cells and Unicode box characters.
    #[must_use]
    pub fn new(width: u16) -> Self {
        Self {
            width,
            tab_width: TAB_WIDTH,
            box_chars: BoxChars::default(),
        }
    }

    /// This context at another width.
    #[must_use]
    pub fn with_width(self, width: u16) -> Self {
        Self { width, ..self }
    }

    /// This context with tab stops every `tab_width` cells.
    #[must_use]
    pub fn with_tab_width(self, tab_width: u16) -> Self {
        Self { tab_width, ..self }
    }

    /// This context with `box_chars` for borders.
    #[must_use]
    pub fn with_box_chars(self, box_chars: BoxChars) -> Self {
        Self { box_chars, ..self }
    }

    /// The width in cells that rendered lines should fit.
    #[must_use]
    pub fn width(self) -> u16 {
        self.width
    }

    /// The spacing of tab stops, in cells.
    #[must_use]
    pub fn tab_width(self) -> u16 {
        self.tab_width
    }

    /// The characters to draw borders and rules with.
    #[must_use]
    pub fn box_chars(self) -> BoxChars {
        self.box_chars
    }
}

/// Content that knows its own size and can draw itself at a width.
///
/// A container asks each child to [`measure`](Self::measure), decides the
/// child's width, and renders the child at that width. The trait is
/// object-safe, so a container can hold `Box<dyn Renderable>`.
///
/// The crate-root docs show a complete widget.
pub trait Renderable {
    /// The narrowest and widest this content can usefully be.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "Context gains fields in later phases; a reference keeps the trait unchanged"
    )]
    fn measure(&self, context: &Context) -> Measurement;

    /// Lines no wider than `context.width()`, except a word or cluster that
    /// the content's overflow policy keeps whole.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "Context gains fields in later phases; a reference keeps the trait unchanged"
    )]
    fn render(&self, context: &Context) -> Vec<Line>;
}

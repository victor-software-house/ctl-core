//! The protocol a widget follows: measure, then render.

use std::num::NonZeroU16;

use crate::cells::TAB_WIDTH;
use crate::line::Line;

/// The narrowest and widest a piece of content can usefully be, in cells.
///
/// The minimum is the widest unit the content will not break: a word under
/// [`Overflow::KeepWords`](crate::Overflow::KeepWords), a cluster under
/// [`Overflow::Fold`](crate::Overflow::Fold). The maximum is the width the
/// content takes when nothing wraps it. The maximum is never below the
/// minimum.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
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
/// It is `Clone` but not `Copy`, so a later release can add a field without
/// breaking a caller that copies it. A widget takes `&Context`.
///
/// The `at_*` and `using_*` methods return a changed clone, so a container
/// derives the context of a child from its own: `context.at_width(column)`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Context {
    width: u16,
    unbounded: bool,
    tab_width: NonZeroU16,
    box_chars: BoxChars,
}

impl Context {
    /// A context `width` cells wide, with tab stops every [`TAB_WIDTH`]
    /// cells and Unicode box characters.
    #[must_use]
    pub fn new(width: u16) -> Self {
        Self {
            width,
            unbounded: false,
            tab_width: TAB_WIDTH,
            box_chars: BoxChars::default(),
        }
    }

    /// A context with no width limit, so a widget takes its natural width.
    ///
    /// [`width`](Self::width) reports `u16::MAX`, so layout arithmetic needs
    /// no special case, and [`is_unbounded`](Self::is_unbounded) tells a
    /// caller that the width is no limit and not a real one. Every method that
    /// returns a changed clone keeps the flag, including
    /// [`at_width`](Self::at_width).
    #[must_use]
    pub fn unbounded() -> Self {
        Self {
            unbounded: true,
            ..Self::new(u16::MAX)
        }
    }

    /// This context at another width.
    #[must_use]
    pub fn at_width(&self, width: u16) -> Self {
        Self {
            width,
            ..self.clone()
        }
    }

    /// This context with tab stops every `tab_width` cells.
    #[must_use]
    pub fn at_tab_width(&self, tab_width: NonZeroU16) -> Self {
        Self {
            tab_width,
            ..self.clone()
        }
    }

    /// This context with `box_chars` for borders.
    #[must_use]
    pub fn using_box_chars(&self, box_chars: BoxChars) -> Self {
        Self {
            box_chars,
            ..self.clone()
        }
    }

    /// The width in cells that rendered lines should fit.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Whether this context was made by [`unbounded`](Self::unbounded), so
    /// [`width`](Self::width) is no limit.
    #[must_use]
    pub fn is_unbounded(&self) -> bool {
        self.unbounded
    }

    /// The spacing of tab stops, in cells.
    #[must_use]
    pub fn tab_width(&self) -> NonZeroU16 {
        self.tab_width
    }

    /// The characters to draw borders and rules with.
    #[must_use]
    pub fn box_chars(&self) -> BoxChars {
        self.box_chars
    }
}

/// Content that knows its own size and can draw itself at a width.
///
/// A container asks each child to [`measure`](Self::measure), decides the
/// child's width, and renders the child at that width. The trait is dyn
/// compatible, so a container can hold `Box<dyn Renderable>`. A reference to a
/// widget and a `Box` of one are widgets too, so a container can borrow its
/// children.
///
/// The `ctl_text` crate docs show a complete widget.
pub trait Renderable {
    /// The narrowest and widest this content can usefully be.
    fn measure(&self, context: &Context) -> Measurement;

    /// Lines no wider than `context.width()`, except a word or cluster that
    /// the content's overflow policy keeps whole.
    fn render(&self, context: &Context) -> Vec<Line>;
}

impl<T: Renderable + ?Sized> Renderable for &T {
    fn measure(&self, context: &Context) -> Measurement {
        (**self).measure(context)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        (**self).render(context)
    }
}

impl<T: Renderable + ?Sized> Renderable for Box<T> {
    fn measure(&self, context: &Context) -> Measurement {
        (**self).measure(context)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        (**self).render(context)
    }
}

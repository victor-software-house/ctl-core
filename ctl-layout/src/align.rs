//! Which edge a cell's text sits against.

/// Which edge of its column a cell's text sits against.
///
/// A [`Column`](crate::Column) applies it to its header and to every cell
/// below it. The other end of a short cell is padded with spaces.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Align {
    /// Text starts at the left edge.
    #[default]
    Left,
    /// Text ends at the right edge. Numbers line up on their last digit.
    Right,
}

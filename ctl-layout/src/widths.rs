//! Choosing the width of each column of a table.

/// What one column can be: the width it takes with nothing wrapped, the
/// narrowest it can be without breaking a word, and the narrowest it can be
/// at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Extent {
    pub(crate) natural: u16,
    pub(crate) minimum: u16,
    pub(crate) floor: u16,
}

/// Whether the table has an outer frame that a wide word would break.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Framing {
    Framed,
    Unframed,
}

/// The content width of each column, given `available` cells for all of them.
///
/// 1. A column starts at its natural width.
/// 2. If the natural widths fit, they stay.
/// 3. Otherwise the widest column that can still shrink gives up one cell,
///    repeatedly, so wide columns come down toward the next widest. A column
///    never goes below its minimum. A tie gives up the rightmost first, so
///    equal columns stay within one cell of each other.
/// 4. If the minimums still do not fit, an unframed table stops: its keep-words
///    columns run past the width. A framed table goes on to the floors, because
///    a frame cannot run past the width and stay aligned.
pub(crate) fn solve(extents: &[Extent], available: u16, framing: Framing) -> Vec<u16> {
    let mut widths: Vec<u16> = extents.iter().map(|extent| extent.natural).collect();
    let minimums: Vec<u16> = extents.iter().map(|extent| extent.minimum).collect();
    shrink(&mut widths, &minimums, available);
    if framing == Framing::Framed {
        let floors: Vec<u16> = extents.iter().map(|extent| extent.floor).collect();
        shrink(&mut widths, &floors, available);
    }
    widths
}

fn shrink(widths: &mut [u16], limits: &[u16], available: u16) {
    let mut total: u32 = widths.iter().map(|width| u32::from(*width)).sum();
    let available = u32::from(available);
    while total > available {
        let widest = widths
            .iter()
            .zip(limits)
            .enumerate()
            .filter(|(_, (width, limit))| width > limit)
            .max_by_key(|(index, (width, _))| (**width, *index))
            .map(|(index, _)| index);
        let Some(index) = widest else {
            return;
        };
        widths[index] = widths[index].saturating_sub(1);
        total -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{Extent, Framing, solve};

    fn extent(natural: u16, minimum: u16) -> Extent {
        Extent {
            natural,
            minimum,
            floor: 1,
        }
    }

    #[test]
    fn natural_widths_that_fit_are_kept() {
        let extents = [extent(8, 8), extent(30, 4)];
        assert_eq!(solve(&extents, 38, Framing::Unframed), [8, 30]);
        assert_eq!(solve(&extents, 80, Framing::Unframed), [8, 30]);
    }

    #[test]
    fn the_widest_column_gives_way_first() {
        let extents = [extent(8, 8), extent(70, 4)];
        assert_eq!(solve(&extents, 33, Framing::Unframed), [8, 25]);
    }

    #[test]
    fn wide_columns_come_down_toward_the_next_widest() {
        let extents = [extent(20, 1), extent(30, 1), extent(40, 1)];
        assert_eq!(solve(&extents, 60, Framing::Unframed), [20, 20, 20]);
        assert_eq!(solve(&extents, 61, Framing::Unframed), [20, 21, 20]);
    }

    #[test]
    fn a_column_never_goes_below_its_minimum() {
        let extents = [extent(30, 30), extent(40, 1)];
        assert_eq!(solve(&extents, 33, Framing::Unframed), [30, 3]);
    }

    #[test]
    fn an_unframed_table_runs_past_the_width_when_minimums_do_not_fit() {
        let extents = [extent(30, 30), extent(40, 12)];
        assert_eq!(solve(&extents, 33, Framing::Unframed), [30, 12]);
    }

    #[test]
    fn a_framed_table_goes_on_to_the_floors() {
        let extents = [extent(30, 30), extent(40, 12)];
        assert_eq!(solve(&extents, 33, Framing::Framed), [21, 12]);
    }

    #[test]
    fn nothing_fits_below_the_floors() {
        let extents = [extent(30, 30), extent(40, 12)];
        assert_eq!(solve(&extents, 1, Framing::Framed), [1, 1]);
    }
}

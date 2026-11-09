//! Choosing the width of each column of a table.

/// What one column can be. `natural` is the width it takes with nothing
/// wrapped, `minimum` the narrowest it can be without breaking a word, and
/// `floor` the narrowest it can be at all.
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
/// 3. Otherwise the widest column that can still shrink gives up cells, so wide
///    columns come down toward the next widest. A column never goes below its
///    minimum. A tie gives up the rightmost first, so equal columns stay within
///    one cell of each other.
/// 4. If the minimums still do not fit, an unframed table stops, and its
///    keep-words columns run past the width. A framed table goes on to the
///    floors, folding only the columns that can fold alone to make it fit. So a
///    short word beside a long URL stays whole.
///
/// These are the widths cells render at. The table then narrows each column
/// to its widest rendered line, step 6 of [`Table`](crate::Table).
pub(crate) fn solve(extents: &[Extent], available: u16, framing: Framing) -> Vec<u16> {
    let mut widths: Vec<u16> = extents.iter().map(|extent| extent.natural).collect();
    let minimums: Vec<u16> = extents.iter().map(|extent| extent.minimum).collect();
    shrink(&mut widths, &minimums, available);
    match framing {
        Framing::Framed => {
            let floors: Vec<u16> = extents.iter().map(|extent| extent.floor).collect();
            let limits = folding_limits(&widths, &floors, available);
            shrink(&mut widths, &limits, available);
        }
        Framing::Unframed => {}
    }
    widths
}

/// The widths the floor pass shrinks `widths` to. A column that can fold to
/// its floor and make the table fit by itself has its floor, and the others
/// keep their width. When none can, every column has its floor.
fn folding_limits(widths: &[u16], floors: &[u16], available: u16) -> Vec<u16> {
    let missing = excess(widths, available);
    let alone = |width: u16, floor: u16| u64::from(width.saturating_sub(floor)) >= missing;
    if widths
        .iter()
        .zip(floors)
        .any(|(width, floor)| alone(*width, *floor))
    {
        widths
            .iter()
            .zip(floors)
            .map(|(width, floor)| {
                if alone(*width, *floor) {
                    *floor
                } else {
                    *width
                }
            })
            .collect()
    } else {
        floors.to_vec()
    }
}

/// How many cells `widths` take beyond `available`.
fn excess(widths: &[u16], available: u16) -> u64 {
    widths
        .iter()
        .fold(0_u64, |total, width| {
            total.saturating_add(u64::from(*width))
        })
        .saturating_sub(u64::from(available))
}

/// Takes cells from the widest columns until `widths` fit in `available` or
/// no column is above its limit.
///
/// The widest group is the columns tied at the widest width. It comes down to
/// the next width in one step: the next widest column's, or a member's limit.
/// When fewer cells are missing than that step takes, the group gives up the
/// same share each, and the rightmost members one more.
fn shrink(widths: &mut [u16], limits: &[u16], available: u16) {
    let mut missing = excess(widths, available);
    while missing > 0 {
        let candidates: Vec<(usize, u16)> = widths
            .iter()
            .zip(limits)
            .enumerate()
            .filter(|(_, (width, limit))| width > limit)
            .map(|(index, (width, _))| (index, *width))
            .collect();
        let Some(widest) = candidates.iter().map(|(_, width)| *width).max() else {
            return;
        };
        let group: Vec<usize> = candidates
            .iter()
            .filter(|(_, width)| *width == widest)
            .map(|(index, _)| *index)
            .collect();
        let next = candidates
            .iter()
            .map(|(_, width)| *width)
            .filter(|width| *width < widest)
            .max()
            .unwrap_or(0);
        let stop = group
            .iter()
            .map(|index| limits[*index])
            .fold(next, u16::max);
        let members = u64::try_from(group.len()).unwrap_or(u64::MAX);
        let cells = u64::from(widest - stop).saturating_mul(members);
        if missing >= cells {
            for index in &group {
                widths[*index] = stop;
            }
            missing -= cells;
        } else {
            let each = u16::try_from(missing / members).unwrap_or(u16::MAX);
            let extra = usize::try_from(missing % members).unwrap_or(usize::MAX);
            for index in &group {
                widths[*index] -= each;
            }
            for index in group.iter().rev().take(extra) {
                widths[*index] -= 1;
            }
            return;
        }
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
    fn a_framed_table_folds_the_column_that_folds_anyway_and_keeps_the_other_word() {
        let extents = [
            Extent {
                natural: 8,
                minimum: 8,
                floor: 1,
            },
            Extent {
                natural: 60,
                minimum: 60,
                floor: 1,
            },
        ];
        assert_eq!(solve(&extents, 13, Framing::Framed), [8, 5]);
    }

    #[test]
    fn columns_that_cannot_fit_alone_fold_together_widest_first() {
        let extents = [extent(30, 30), extent(40, 12), extent(10, 10)];
        assert_eq!(solve(&extents, 20, Framing::Framed), [7, 7, 6]);
    }

    #[test]
    fn the_widest_widths_come_down_in_one_step() {
        let extents = [extent(u16::MAX, 1), extent(u16::MAX, 1)];
        assert_eq!(solve(&extents, 2, Framing::Unframed), [1, 1]);
    }

    #[test]
    fn nothing_fits_below_the_floors() {
        let extents = [extent(30, 30), extent(40, 12)];
        assert_eq!(solve(&extents, 1, Framing::Framed), [1, 1]);
    }
}

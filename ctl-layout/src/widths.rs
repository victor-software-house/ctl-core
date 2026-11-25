//! Choosing the width of each column of a table.

/// What one column can be. `natural` is the width it takes with nothing
/// wrapped, `minimum` the narrowest it can be without breaking a word, and
/// `floor` the narrowest it can be at all, the widest cluster. `column_floor`
/// is the width no column goes below, whatever its overflow: see
/// [`column_floor`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Extent {
    pub(crate) natural: u16,
    pub(crate) minimum: u16,
    pub(crate) floor: u16,
    pub(crate) column_floor: u16,
}

impl Extent {
    /// The narrowest this column gets in a table with `framing`.
    ///
    /// It is the larger of the column floor and the limit the overflow sets:
    /// the widest word for an unframed table, the widest cluster for a framed
    /// one. Both passes of [`solve`], the narrowest width a table measures,
    /// and [`Table::fits`](crate::Table::fits) all read it, so one rule
    /// serves them.
    pub(crate) fn limit(self, framing: Framing) -> u16 {
        match framing {
            Framing::Framed => self.floor.max(self.column_floor),
            Framing::Unframed => self.minimum.max(self.column_floor),
        }
    }
}

/// The column floor of a column `natural` cells wide under a header `header`
/// cells wide, given the `setting`.
///
/// It is 0 when the setting is 0. Otherwise it is the larger of the header
/// width and the setting, and never more than the natural width, so a short
/// column is not padded.
pub(crate) fn column_floor(natural: u16, header: u16, setting: u16) -> u16 {
    if setting == 0 {
        0
    } else {
        natural.min(header.max(setting))
    }
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
///    limit, the larger of its minimum and its column floor. A tie gives up the
///    rightmost first, so equal columns stay within one cell of each other.
/// 4. If those limits still do not fit, an unframed table stops, and its
///    columns run past the width at their limits. A framed table goes on to the
///    fold limits, the larger of the widest cluster and the column floor,
///    folding only the columns that can fold alone to make it fit. So a short
///    word beside a long URL stays whole.
///
/// These are the widths cells render at. The table then narrows each column
/// to its widest rendered line, step 6 of [`Table`](crate::Table).
pub(crate) fn solve(extents: &[Extent], available: u16, framing: Framing) -> Vec<u16> {
    let mut widths: Vec<u16> = extents.iter().map(|extent| extent.natural).collect();
    let minimums: Vec<u16> = extents
        .iter()
        .map(|extent| extent.limit(Framing::Unframed))
        .collect();
    shrink(&mut widths, &minimums, available);
    match framing {
        Framing::Framed => {
            let floors: Vec<u16> = extents
                .iter()
                .map(|extent| extent.limit(Framing::Framed))
                .collect();
            let limits = folding_limits(&widths, &floors, available);
            shrink(&mut widths, &limits, available);
        }
        Framing::Unframed => {}
    }
    widths
}

/// The widths the fold pass shrinks `widths` to. A column that can fold to
/// its fold limit and make the table fit by itself has it, and the others
/// keep their width. When none can, every column has its fold limit.
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
    use ctl_text::Context;

    use super::{Extent, Framing, column_floor, solve};
    use crate::{Column, Frame, Table};

    fn extent(natural: u16, minimum: u16) -> Extent {
        Extent {
            natural,
            minimum,
            floor: 1,
            column_floor: 0,
        }
    }

    fn floored(natural: u16, minimum: u16, column_floor: u16) -> Extent {
        Extent {
            column_floor,
            ..extent(natural, minimum)
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
                column_floor: 0,
            },
            Extent {
                natural: 60,
                minimum: 60,
                floor: 1,
                column_floor: 0,
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

    #[test]
    fn the_column_floor_is_the_header_or_the_setting_and_never_past_the_natural_width() {
        assert_eq!(
            column_floor(30, 12, 8),
            12,
            "a header wider than the setting"
        );
        assert_eq!(
            column_floor(30, 4, 8),
            8,
            "a header narrower than the setting"
        );
        assert_eq!(
            column_floor(3, 3, 8),
            3,
            "a natural width below the setting"
        );
        assert_eq!(column_floor(30, 0, 8), 8, "no header");
        assert_eq!(column_floor(30, 12, 0), 0, "the setting off");
        assert_eq!(column_floor(30, 4, 12), 12, "a larger setting");
    }

    #[test]
    fn the_shrink_pass_stops_at_the_column_floor() {
        let extents = [floored(30, 3, 8), floored(40, 1, 8)];
        assert_eq!(solve(&extents, 10, Framing::Unframed), [8, 8]);
        assert_eq!(solve(&extents, 20, Framing::Unframed), [10, 10]);
    }

    #[test]
    fn a_column_floor_below_the_minimum_changes_nothing() {
        let extents = [floored(30, 30, 8), floored(40, 1, 8)];
        assert_eq!(solve(&extents, 33, Framing::Unframed), [30, 8]);
    }

    #[test]
    fn the_fold_pass_stops_at_the_column_floor() {
        let extents = [floored(30, 30, 8), floored(40, 12, 8)];
        assert_eq!(solve(&extents, 5, Framing::Framed), [8, 8]);
        assert_eq!(solve(&extents, 33, Framing::Framed), [21, 12]);
    }

    #[test]
    fn an_ellipsis_column_stops_at_its_column_floor() {
        let extents = [floored(30, 1, 8)];
        assert_eq!(solve(&extents, 3, Framing::Unframed), [8]);
        assert_eq!(solve(&extents, 3, Framing::Framed), [8]);
        assert_eq!(solve(&[extent(30, 1)], 3, Framing::Unframed), [3]);
    }

    #[test]
    fn a_column_floor_of_zero_leaves_the_limits_as_they_are() {
        let extents = [floored(30, 30, 0), floored(40, 1, 0)];
        assert_eq!(solve(&extents, 33, Framing::Unframed), [30, 3]);
        assert_eq!(extents[1].limit(Framing::Unframed), 1);
        assert_eq!(extents[1].limit(Framing::Framed), 1);
    }

    /// An unframed table needs 8 for the id, 10 for the word "atomically",
    /// and a gap of 2. A grid needs two fold limits of 8 and a frame of 7. With
    /// the floor off, a grid column's limit is its widest cluster, 1.
    #[test]
    fn a_table_fits_when_its_limits_and_frame_fit_the_width() {
        let table = |frame| {
            Table::new([Column::new("id"), Column::new("title")])
                .frame(frame)
                .row(["QCTL-014", "Write a ledger atomically"])
        };
        assert!(table(Frame::Plain).fits(&Context::new(20)));
        assert!(!table(Frame::Plain).fits(&Context::new(19)));
        assert!(table(Frame::Grid).fits(&Context::new(23)));
        assert!(!table(Frame::Grid).fits(&Context::new(22)));
        assert!(table(Frame::Grid).column_floor(0).fits(&Context::new(9)));
        assert!(!table(Frame::Grid).column_floor(0).fits(&Context::new(8)));
    }

    #[test]
    fn a_blank_table_fits_any_width() {
        let table = Table::new([Column::untitled(), Column::untitled()]);
        assert!(table.fits(&Context::new(0)));
    }
}

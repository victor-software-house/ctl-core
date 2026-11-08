//! A table of rows and columns.

use ctl_text::{Context, Line, Measurement, Overflow, Renderable, Span, Style, Text};

use crate::align::Align;
use crate::column::Column;
use crate::frame::{Frame, RowSeparation};
use crate::glyphs::Glyphs;
use crate::lines::{indent_lines, literal, push_cell, rule_line};
use crate::widths::{Extent, Framing, solve};

/// Cells between two columns of an unframed table.
const GAP: u16 = 2;
/// The indent of a stacked row's label lines.
const LABEL_INDENT: u16 = 2;
/// The indent of a stacked row's description lines.
const DESCRIPTION_INDENT: u16 = 4;

#[derive(Clone, Copy, Debug)]
struct Stacked {
    threshold: u16,
    label_columns: usize,
}

/// A table of rows under column headers.
///
/// A table measures its columns, gives each one a width, and renders every
/// cell at its column's width:
///
/// 1. Each column starts at the widest cell it holds.
/// 2. When those widths and the frame fit, the table is exactly that wide.
/// 3. Otherwise the widest column that can still shrink gives up a cell, again
///    and again, never below the widest word of a keep-words column or the
///    widest cluster of a fold column. An ellipsis column can shrink to 1 cell.
/// 4. When the narrowest widths still do not fit, an unframed table lets its
///    keep-words columns run past the width. A [`Frame::Grid`] table folds
///    those columns instead, so its frame stays aligned.
/// 5. Each cell renders at its column's width. An ellipsis cell ends in `…`,
///    and a numeric column aligns right.
///
/// A table with [`stacked_below`](Self::stacked_below) renders each row as
/// indented lines instead when the width is under its threshold.
#[derive(Clone, Debug)]
pub struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<Text>>,
    frame: Frame,
    separation: RowSeparation,
    stacked: Option<Stacked>,
}

impl Table {
    /// A table with `columns` and no rows, drawn as a [`Frame::Grid`].
    #[must_use]
    pub fn new(columns: impl IntoIterator<Item = Column>) -> Self {
        Self::from_columns(columns.into_iter().collect())
    }

    fn from_columns(columns: Vec<Column>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            frame: Frame::default(),
            separation: RowSeparation::default(),
            stacked: None,
        }
    }

    /// Draws the table with `frame`.
    #[must_use]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = frame;
        self
    }

    /// Separates consecutive rows with `separation`.
    #[must_use]
    pub fn row_separation(mut self, separation: RowSeparation) -> Self {
        self.separation = separation;
        self
    }

    /// Renders each row as indented lines when the width is under
    /// `threshold`: the first `label_columns` cells on a line indented by 2,
    /// and the rest below them, indented by 4. A stacked table has no frame
    /// and no header.
    #[must_use]
    pub fn stacked_below(mut self, threshold: u16, label_columns: usize) -> Self {
        self.stacked = Some(Stacked {
            threshold,
            label_columns,
        });
        self
    }

    /// Adds a row. A row with fewer cells than the table has columns gets
    /// empty cells, and cells past the last column are dropped.
    #[must_use]
    pub fn row(self, cells: impl IntoIterator<Item = Text>) -> Self {
        self.push_row(cells.into_iter().collect())
    }

    /// Adds every row of `rows`, as [`row`](Self::row) adds one.
    #[must_use]
    pub fn rows<R>(self, rows: impl IntoIterator<Item = R>) -> Self
    where
        R: IntoIterator<Item = Text>,
    {
        rows.into_iter().fold(self, Self::row)
    }

    fn push_row(mut self, mut cells: Vec<Text>) -> Self {
        cells.resize_with(self.columns.len(), Text::new);
        self.rows.push(cells);
        self
    }

    fn has_header(&self) -> bool {
        self.columns.iter().any(|column| column.header.is_some())
    }

    fn framing(&self) -> Framing {
        if self.frame == Frame::Grid {
            Framing::Framed
        } else {
            Framing::Unframed
        }
    }

    /// Cells taken by the frame and the gaps, for the columns it has.
    fn frame_width(&self) -> u16 {
        let count = u16::try_from(self.columns.len()).unwrap_or(u16::MAX);
        if self.frame == Frame::Grid {
            count.saturating_mul(3).saturating_add(1)
        } else {
            count.saturating_sub(1).saturating_mul(GAP)
        }
    }

    /// The stacking that applies at `context`'s width, if any.
    fn stacking(&self, context: Context) -> Option<Stacked> {
        self.stacked
            .filter(|stacked| context.width() < stacked.threshold)
    }

    /// Every cell, the header row first when the table has one, each with its
    /// column's declared overflow.
    fn cells(&self) -> Vec<Vec<Text>> {
        let with_overflow = |cells: &[Text]| -> Vec<Text> {
            cells
                .iter()
                .zip(&self.columns)
                .map(|(cell, column)| cell.clone().overflow(column.overflow))
                .collect()
        };
        let mut grid = Vec::new();
        if self.has_header() {
            let header: Vec<Text> = self
                .columns
                .iter()
                .map(|column| column.header.clone().unwrap_or_default())
                .collect();
            grid.push(with_overflow(&header));
        }
        grid.extend(self.rows.iter().map(|row| with_overflow(row)));
        grid
    }

    fn extents(&self, grid: &[Vec<Text>], context: Context) -> Vec<Extent> {
        (0..self.columns.len())
            .map(|index| {
                let mut extent = Extent {
                    natural: 0,
                    minimum: 0,
                    floor: 0,
                };
                for cell in grid.iter().filter_map(|row| row.get(index)) {
                    let measured = cell.measure(&context);
                    let folded = cell.clone().overflow(Overflow::Fold).measure(&context);
                    extent.natural = extent.natural.max(measured.maximum());
                    extent.minimum = extent.minimum.max(measured.minimum());
                    extent.floor = extent.floor.max(folded.minimum().min(measured.minimum()));
                }
                extent
            })
            .collect()
    }

    /// Rows of cells rendered at `widths`. A grid folds, so a keep-words
    /// column renders with [`Overflow::Fold`].
    fn render_cells(
        &self,
        grid: &[Vec<Text>],
        widths: &[u16],
        context: Context,
    ) -> Vec<Vec<Vec<Line>>> {
        grid.iter()
            .map(|row| {
                row.iter()
                    .zip(widths)
                    .zip(&self.columns)
                    .map(|((cell, width), column)| {
                        let fold =
                            self.frame == Frame::Grid && column.overflow == Overflow::KeepWords;
                        let cell = if fold {
                            cell.clone().overflow(Overflow::Fold)
                        } else {
                            cell.clone()
                        };
                        cell.render(&context.with_width(*width))
                    })
                    .collect()
            })
            .collect()
    }

    fn row_lines(&self, cells: &[Vec<Line>], widths: &[u16], glyphs: &Glyphs) -> Vec<Line> {
        let height = cells.iter().map(Vec::len).max().unwrap_or(0).max(1);
        let empty = Line::new();
        (0..height)
            .map(|index| {
                let mut line = Line::new();
                let framed = self.frame == Frame::Grid;
                if framed {
                    line.push(literal(&glyphs.outer.to_string()));
                }
                for (column, width) in widths.iter().enumerate() {
                    if column > 0 {
                        line.push(literal(&if framed {
                            glyphs.inner.to_string()
                        } else {
                            " ".repeat(usize::from(GAP))
                        }));
                    }
                    if framed {
                        line.push(literal(" "));
                    }
                    let cell = cells
                        .get(column)
                        .and_then(|lines| lines.get(index))
                        .unwrap_or(&empty);
                    let align = self.columns.get(column).map_or(Align::Left, |c| c.align);
                    push_cell(&mut line, cell, *width, align);
                    if framed {
                        line.push(literal(" "));
                    }
                }
                if framed {
                    line.push(literal(&glyphs.outer.to_string()));
                } else {
                    line.trim_end();
                }
                line
            })
            .collect()
    }

    fn compose(&self, cells: &[Vec<Vec<Line>>], widths: &[u16], glyphs: &Glyphs) -> Vec<Line> {
        let header = self.has_header();
        let framed = self.frame == Frame::Grid;
        let total = widths
            .iter()
            .fold(self.frame_width(), |sum, width| sum.saturating_add(*width));
        let blank_cells = vec![Vec::new(); widths.len()];
        let mut lines = Vec::new();
        if framed {
            lines.push(glyphs.top.line(widths));
        }
        for (index, row) in cells.iter().enumerate() {
            if header && index == 1 {
                match self.frame {
                    Frame::Grid => lines.push(glyphs.header.line(widths)),
                    Frame::HeaderRule => lines.push(rule_line(glyphs.rule, total)),
                    Frame::Plain => {}
                }
            } else if index > 0 {
                match self.separation {
                    RowSeparation::Rule if framed => lines.push(glyphs.row.line(widths)),
                    RowSeparation::Rule => lines.push(rule_line(glyphs.rule, total)),
                    RowSeparation::Blank => {
                        lines.extend(self.row_lines(&blank_cells, widths, glyphs));
                    }
                    RowSeparation::None => {}
                }
            }
            lines.extend(self.row_lines(row, widths, glyphs));
        }
        if framed {
            lines.push(glyphs.bottom.line(widths));
        }
        lines
    }

    /// The stacked form of every row, laid out at `width`.
    fn stacked_lines(&self, context: Context, width: u16, stacked: Stacked) -> Vec<Line> {
        let tab_width = context.tab_width();
        let mut lines = Vec::new();
        for row in &self.rows {
            let labels = join_cells(row.iter().take(stacked.label_columns), tab_width);
            let description = join_cells(row.iter().skip(stacked.label_columns), tab_width);
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

/// The non-empty cells on one line, separated by a space.
fn join_cells<'a>(cells: impl Iterator<Item = &'a Text>, tab_width: u16) -> Line {
    let mut joined = Line::new();
    for line in cells
        .flat_map(|cell| cell.lines(tab_width))
        .filter(|line| !line.is_empty())
    {
        if !joined.is_empty() {
            joined.push(Span::new(" ", Style::new()));
        }
        for span in line.spans() {
            joined.push(span.clone());
        }
    }
    joined
}

impl Renderable for Table {
    fn measure(&self, context: &Context) -> Measurement {
        if let Some(stacked) = self.stacking(*context) {
            let widest = |width| {
                self.stacked_lines(*context, width, stacked)
                    .iter()
                    .map(Line::width)
                    .max()
                    .unwrap_or(0)
            };
            return Measurement::new(widest(1), widest(u16::MAX));
        }
        let extents = self.extents(&self.cells(), *context);
        let frame = u32::from(self.frame_width());
        let sum = |value: fn(&Extent) -> u16| {
            extents
                .iter()
                .fold(frame, |total, extent| total + u32::from(value(extent)))
        };
        let narrowest = if self.frame == Frame::Grid {
            sum(|extent| extent.floor)
        } else {
            sum(|extent| extent.minimum)
        };
        let natural = sum(|extent| extent.natural);
        Measurement::new(
            u16::try_from(narrowest).unwrap_or(u16::MAX),
            u16::try_from(natural).unwrap_or(u16::MAX),
        )
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        if self.columns.is_empty() {
            return Vec::new();
        }
        if let Some(stacked) = self.stacking(*context) {
            return self.stacked_lines(*context, context.width(), stacked);
        }
        let grid = self.cells();
        let extents = self.extents(&grid, *context);
        let available = context.width().saturating_sub(self.frame_width());
        let widths = solve(&extents, available, self.framing());
        let cells = self.render_cells(&grid, &widths, *context);
        let glyphs = Glyphs::for_chars(context.box_chars());
        self.compose(&cells, &widths, &glyphs)
    }
}

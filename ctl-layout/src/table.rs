//! A table of rows and columns.

use ctl_text::{Context, Line, Measurement, Overflow, Renderable, Text};

use crate::column::Column;
use crate::frame::{Frame, RowSeparation};
use crate::glyphs::Glyphs;
use crate::lines::{literal, push_cell, rule_line};
use crate::widths::{Extent, Framing, solve};

/// Cells between two columns of an unframed table.
const GAP: u16 = 2;

/// A table of rows under column headers.
///
/// A table measures its columns, gives each one a width, and renders every
/// cell at its column's width:
///
/// 1. Each column starts at the widest cell it holds.
/// 2. When those widths and the frame fit, the table is exactly that wide.
/// 3. Otherwise the widest column that can still shrink gives up a cell, again
///    and again. A keep-words column never goes below its widest word, and a
///    fold column never goes below its widest cluster. An ellipsis column can
///    shrink to 1 cell.
/// 4. When the narrowest widths still do not fit, an unframed table lets its
///    keep-words columns run past the width. A [`Frame::Grid`] table folds
///    those columns instead, so its frame stays aligned. It folds only the
///    columns that can fold alone to make it fit, so a short id keeps its word
///    beside a long URL.
/// 5. Each cell renders at its column's width. An ellipsis cell ends in `…`,
///    and a numeric column aligns right.
/// 6. Each column then narrows to its widest rendered line. A column that
///    wrapped holds no blank cells down every row, and a table sized to its
///    content draws no wider than it needs. An ellipsis column that cut a cell
///    keeps its width, since the cut cell fills it.
///
/// Narrowing never changes the lines: text wrapped at a width wraps the same
/// at its own widest line.
///
/// A table with no rows and no header draws nothing and measures 0.
#[derive(Clone, Debug)]
pub struct Table {
    columns: Vec<Column>,
    /// The headers of the columns, moved out of them, with each column's
    /// overflow applied. `None` when no column has a header.
    header: Option<Vec<Text>>,
    /// Every row has one cell per column, each with its column's overflow.
    rows: Vec<Vec<Text>>,
    frame: Frame,
    separation: RowSeparation,
}

impl Table {
    /// A table with `columns` and no rows, drawn as a [`Frame::Grid`] with no
    /// separation between rows.
    #[must_use]
    pub fn new(columns: impl IntoIterator<Item = Column>) -> Self {
        Self::from_columns(columns.into_iter().collect())
    }

    fn from_columns(mut columns: Vec<Column>) -> Self {
        let titled = columns.iter().any(|column| column.header.is_some());
        let header: Option<Vec<Text>> = titled.then(|| {
            columns
                .iter_mut()
                .map(|column| {
                    column
                        .header
                        .take()
                        .unwrap_or_default()
                        .overflow(column.overflow)
                })
                .collect()
        });
        Self {
            columns,
            header,
            rows: Vec::new(),
            frame: Frame::default(),
            separation: RowSeparation::default(),
        }
    }

    /// Sets what the table draws around and between its columns, replacing
    /// the default [`Frame::Grid`].
    #[must_use]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = frame;
        self
    }

    /// Sets what separates consecutive rows, replacing the default
    /// [`RowSeparation::None`].
    #[must_use]
    pub fn row_separation(mut self, separation: RowSeparation) -> Self {
        self.separation = separation;
        self
    }

    /// Adds a row of `&str`, `String`, [`Text`], or anything else that
    /// converts to a [`Text`].
    ///
    /// The column's overflow replaces the cell's own. A row with fewer cells
    /// than the table has columns gets empty cells, and cells past the last
    /// column are dropped.
    #[must_use]
    pub fn row<C: Into<Text>>(self, cells: impl IntoIterator<Item = C>) -> Self {
        self.push_row(cells.into_iter().map(Into::into).collect())
    }

    /// Adds every row of `rows` in order, as [`row`](Self::row) adds one.
    #[must_use]
    pub fn rows<C, R>(self, rows: impl IntoIterator<Item = R>) -> Self
    where
        R: IntoIterator<Item = C>,
        C: Into<Text>,
    {
        rows.into_iter().fold(self, Table::row)
    }

    fn push_row(mut self, mut cells: Vec<Text>) -> Self {
        cells.resize_with(self.columns.len(), Text::new);
        let cells = cells
            .into_iter()
            .zip(&self.columns)
            .map(|(cell, column)| cell.overflow(column.overflow))
            .collect();
        self.rows.push(cells);
        self
    }

    /// The rows, so a [`Record`](crate::Record) can change its frame without
    /// losing them.
    pub(crate) fn into_rows(self) -> Vec<Vec<Text>> {
        self.rows
    }

    /// Whether there is nothing to draw: no columns, or neither a header nor
    /// a row. A grid would otherwise draw an empty box.
    fn is_blank(&self) -> bool {
        self.columns.is_empty() || (self.header.is_none() && self.rows.is_empty())
    }

    /// Cells taken by the frame and the gaps, for the columns it has.
    fn frame_width(&self) -> u16 {
        let count = u16::try_from(self.columns.len()).unwrap_or(u16::MAX);
        match self.frame.framing() {
            Framing::Framed => count.saturating_mul(3).saturating_add(1),
            Framing::Unframed => count.saturating_sub(1).saturating_mul(GAP),
        }
    }

    /// The header row, when there is one, then every row.
    fn grid(&self) -> impl Iterator<Item = &[Text]> {
        self.header.iter().chain(&self.rows).map(Vec::as_slice)
    }

    /// The overflow a cell of `column` renders with. A grid folds, so a
    /// keep-words column folds a word wider than itself and the frame stays
    /// aligned.
    fn cell_overflow(&self, column: &Column) -> Overflow {
        match self.frame.framing() {
            Framing::Framed if column.overflow == Overflow::KeepWords => Overflow::Fold,
            Framing::Framed | Framing::Unframed => column.overflow,
        }
    }

    fn extents(&self, context: &Context) -> Vec<Extent> {
        let framing = self.frame.framing();
        self.columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let mut extent = Extent {
                    natural: 0,
                    minimum: 0,
                    floor: 0,
                };
                for cell in self.grid().filter_map(|row| row.get(index)) {
                    let measured = cell.measure(context);
                    let floor = match framing {
                        Framing::Framed if column.overflow != Overflow::Fold => cell
                            .measure_as(context, Overflow::Fold)
                            .minimum()
                            .min(measured.minimum()),
                        Framing::Framed | Framing::Unframed => measured.minimum(),
                    };
                    extent.natural = extent.natural.max(measured.maximum());
                    extent.minimum = extent.minimum.max(measured.minimum());
                    extent.floor = extent.floor.max(floor);
                }
                extent
            })
            .collect()
    }

    /// Rows of cells rendered at `widths`.
    fn render_cells(&self, widths: &[u16], context: &Context) -> Vec<Vec<Vec<Line>>> {
        self.grid()
            .map(|row| {
                row.iter()
                    .zip(widths)
                    .zip(&self.columns)
                    .map(|((cell, width), column)| {
                        let context = context.at_width(*width);
                        cell.render_as(&context, self.cell_overflow(column))
                    })
                    .collect()
            })
            .collect()
    }

    fn row_lines(&self, cells: &[Vec<Line>], widths: &[u16], glyphs: &Glyphs) -> Vec<Line> {
        let height = cells.iter().map(Vec::len).max().unwrap_or(0).max(1);
        let empty = Line::new();
        let framing = self.frame.framing();
        let (edge, gap, padding) = match framing {
            Framing::Framed => (
                glyphs.outer.to_string(),
                glyphs.inner.to_string(),
                " ".to_owned(),
            ),
            Framing::Unframed => (String::new(), " ".repeat(usize::from(GAP)), String::new()),
        };
        (0..height)
            .map(|index| {
                let mut line = Line::from(literal(&edge));
                for (column, (width, spec)) in widths.iter().zip(&self.columns).enumerate() {
                    if column > 0 {
                        line.push(literal(&gap));
                    }
                    line.push(literal(&padding));
                    let cell = cells
                        .get(column)
                        .and_then(|lines| lines.get(index))
                        .unwrap_or(&empty);
                    push_cell(&mut line, cell, *width, spec.align);
                    line.push(literal(&padding));
                }
                match framing {
                    Framing::Framed => line.push(literal(&edge)),
                    Framing::Unframed => line.trim_end(),
                }
                line
            })
            .collect()
    }

    fn compose(&self, cells: &[Vec<Vec<Line>>], widths: &[u16], glyphs: &Glyphs) -> Vec<Line> {
        let header = self.header.is_some();
        let framing = self.frame.framing();
        let total = widths
            .iter()
            .fold(self.frame_width(), |sum, width| sum.saturating_add(*width));
        let blank_cells = vec![Vec::new(); widths.len()];
        let mut lines = Vec::new();
        match framing {
            Framing::Framed => lines.push(glyphs.top.line(widths)),
            Framing::Unframed => {}
        }
        for (index, row) in cells.iter().enumerate() {
            if header && index == 1 {
                match self.frame {
                    Frame::Grid => lines.push(glyphs.header.line(widths)),
                    Frame::HeaderRule => lines.push(rule_line(glyphs.rule, total)),
                    Frame::Plain => {}
                }
            } else if index > 0 {
                match (self.separation, framing) {
                    (RowSeparation::Rule, Framing::Framed) => lines.push(glyphs.row.line(widths)),
                    (RowSeparation::Rule, Framing::Unframed) => {
                        lines.push(rule_line(glyphs.rule, total));
                    }
                    (RowSeparation::Blank, Framing::Framed | Framing::Unframed) => {
                        lines.extend(self.row_lines(&blank_cells, widths, glyphs));
                    }
                    (RowSeparation::None, Framing::Framed | Framing::Unframed) => {}
                }
            }
            lines.extend(self.row_lines(row, widths, glyphs));
        }
        match framing {
            Framing::Framed => lines.push(glyphs.bottom.line(widths)),
            Framing::Unframed => {}
        }
        lines
    }
}

/// Each of `widths` lowered to the widest line its column rendered, step 6 of
/// [`Table`]. A width never grows: a word kept whole past its column's width
/// runs past it as before.
fn narrowed(cells: &[Vec<Vec<Line>>], widths: &[u16]) -> Vec<u16> {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            cells
                .iter()
                .filter_map(|row| row.get(index))
                .flatten()
                .map(Line::width)
                .max()
                .unwrap_or(0)
                .min(*width)
        })
        .collect()
}

impl Renderable for Table {
    fn measure(&self, context: &Context) -> Measurement {
        if self.is_blank() {
            return Measurement::exact(0);
        }
        let extents = self.extents(context);
        let frame = u64::from(self.frame_width());
        let sum = |value: fn(&Extent) -> u16| {
            extents.iter().fold(frame, |total, extent| {
                total.saturating_add(u64::from(value(extent)))
            })
        };
        let narrowest = match self.frame.framing() {
            Framing::Framed => sum(|extent| extent.floor),
            Framing::Unframed => sum(|extent| extent.minimum),
        };
        let natural = sum(|extent| extent.natural);
        Measurement::new(
            u16::try_from(narrowest).unwrap_or(u16::MAX),
            u16::try_from(natural).unwrap_or(u16::MAX),
        )
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        if self.is_blank() {
            return Vec::new();
        }
        let extents = self.extents(context);
        let available = context.width().saturating_sub(self.frame_width());
        let widths = solve(&extents, available, self.frame.framing());
        let cells = self.render_cells(&widths, context);
        let widths = narrowed(&cells, &widths);
        let glyphs = Glyphs::for_chars(context.box_chars());
        self.compose(&cells, &widths, &glyphs)
    }
}

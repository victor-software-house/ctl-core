//! Building lines out of cells, spaces, and rules.

use ctl_text::{Line, Span, Style};

use crate::align::Align;

/// A span of `text` in the default style.
pub(crate) fn literal(text: &str) -> Span {
    Span::new(text, Style::new())
}

/// A span of `count` spaces. Zero spaces add nothing to a line.
pub(crate) fn spaces(count: u16) -> Span {
    literal(&" ".repeat(usize::from(count)))
}

/// A line of `count` repeats of `fill`.
pub(crate) fn rule_line(fill: char, count: u16) -> Line {
    let text: String = std::iter::repeat_n(fill, usize::from(count)).collect();
    Line::from(literal(&text))
}

/// Adds `cell` to `target`, padded with spaces to `width` on the side
/// `align` leaves free.
pub(crate) fn push_cell(target: &mut Line, cell: &Line, width: u16, align: Align) {
    let gap = spaces(width.saturating_sub(cell.width()));
    match align {
        Align::Left => {
            target.extend(cell.spans().iter().cloned());
            target.push(gap);
        }
        Align::Right => {
            target.push(gap);
            target.extend(cell.spans().iter().cloned());
        }
    }
}

/// `lines` moved right by `columns` cells. A blank line stays blank, so no
/// line ends in a space.
pub(crate) fn indent_lines(lines: Vec<Line>, columns: u16) -> Vec<Line> {
    lines
        .into_iter()
        .map(|line| {
            if line.is_empty() {
                return Line::new();
            }
            let mut indented = Line::from(spaces(columns));
            indented.extend(line.spans().iter().cloned());
            indented
        })
        .collect()
}

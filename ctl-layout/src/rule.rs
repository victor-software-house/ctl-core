//! A horizontal rule, with an optional title.

use ctl_text::{Context, Line, Measurement, Overflow, Renderable, Text};

use crate::glyphs::Glyphs;
use crate::lines::{literal, rule_line};

/// Cells a titled rule spends besides the title: two rule characters and a
/// space before it, and a space after it.
const TITLE_MARGIN: u16 = 4;

/// A rule across the whole width, drawn with `─`, or `-` for
/// [`BoxChars::Ascii`](ctl_text::BoxChars::Ascii). A title sits at the left:
///
/// ```text
/// ── Title ──────────────
/// ```
///
/// A rule has no natural width. It measures as its narrowest, and fills any
/// width it is given. A title that does not fit ends in `…`, and only the
/// first line of a multi-line title is drawn.
#[derive(Clone, Debug, Default)]
pub struct Rule {
    title: Option<Text>,
}

impl Rule {
    /// A rule with no title.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A rule with `title` at its left.
    #[must_use]
    pub fn titled(title: Text) -> Self {
        Self { title: Some(title) }
    }

    fn title(&self) -> Option<Text> {
        self.title
            .clone()
            .map(|title| title.overflow(Overflow::Ellipsis))
    }
}

impl Renderable for Rule {
    fn measure(&self, context: &Context) -> Measurement {
        let Some(title) = self.title() else {
            return Measurement::exact(1);
        };
        let measured = title.measure(context);
        Measurement::new(
            measured.minimum().saturating_add(TITLE_MARGIN),
            measured.maximum().saturating_add(TITLE_MARGIN),
        )
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        let rule_char = Glyphs::for_chars(context.box_chars()).rule;
        let width = context.width();
        let titled = self.title().and_then(|title| {
            let room = width.saturating_sub(TITLE_MARGIN);
            if room == 0 {
                return None;
            }
            title
                .render(&context.with_width(room))
                .into_iter()
                .find(|line| !line.is_empty())
        });
        let Some(title) = titled else {
            return vec![rule_line(rule_char, width)];
        };
        let mut line = Line::new();
        line.push(literal(&format!("{rule_char}{rule_char} ")));
        for span in title.spans() {
            line.push(span.clone());
        }
        let fill = width.saturating_sub(title.width().saturating_add(TITLE_MARGIN));
        if fill > 0 {
            let tail: String = std::iter::once(' ')
                .chain(std::iter::repeat_n(rule_char, usize::from(fill)))
                .collect();
            line.push(literal(&tail));
        }
        vec![line]
    }
}

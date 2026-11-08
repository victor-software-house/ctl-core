//! The only place an escape sequence is written.

use anstyle::Style;
use ctl_text::Line;

use crate::color::ColorDepth;

/// The SGR sequence that clears every attribute.
pub const RESET: &str = "\u{1b}[0m";

/// Turns lines of styled spans into the bytes a terminal reads.
///
/// Layout never sees an escape byte: it produces [`Line`]s, and this encoder
/// adds the styling afterwards. It writes a style sequence only where the
/// style changes between spans, with every colour converted to the encoder's
/// depth, and it ends a line with a reset only when the line wrote a style,
/// so each line stands alone in `head`, `less -R`, and a log.
///
/// An encoder with no depth writes the text of each line byte for byte, with
/// no escape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Encoder {
    depth: Option<ColorDepth>,
}

impl Encoder {
    /// An encoder that writes colour at `depth`, or none for `None`.
    #[must_use]
    pub const fn new(depth: Option<ColorDepth>) -> Self {
        Self { depth }
    }

    /// `lines` as text, each line ended by a newline.
    ///
    /// Each line starts with no style on and ends with none on.
    #[must_use]
    pub fn encode(self, lines: &[Line]) -> String {
        let mut out = String::new();
        for line in lines {
            self.encode_line(line, &mut out);
            out.push('\n');
        }
        out
    }

    fn encode_line(self, line: &Line, out: &mut String) {
        let Some(depth) = self.depth else {
            for span in line.spans() {
                out.push_str(span.text());
            }
            return;
        };
        let mut current = Style::new();
        let mut wrote_style = false;
        for span in line.spans() {
            let style = fit(depth, span.style());
            if style != current {
                if !current.is_plain() {
                    out.push_str(RESET);
                }
                if !style.is_plain() {
                    out.push_str(&style.render().to_string());
                    wrote_style = true;
                }
                current = style;
            }
            out.push_str(span.text());
        }
        if wrote_style {
            out.push_str(RESET);
        }
    }
}

/// `style` with every colour converted to the nearest one `depth` can show.
fn fit(depth: ColorDepth, style: Style) -> Style {
    Style::new()
        .fg_color(style.get_fg_color().map(|color| depth.downgrade(color)))
        .bg_color(style.get_bg_color().map(|color| depth.downgrade(color)))
        .underline_color(
            style
                .get_underline_color()
                .map(|color| depth.downgrade(color)),
        )
        .effects(style.get_effects())
}

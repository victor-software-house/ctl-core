//! Styled text for terminals: spans that are safe to print, widths counted in
//! cells, and lines wrapped or cut to fit.
//!
//! Layout works on plain data. A [`Span`] is text in an [`anstyle::Style`], a
//! [`Line`] is spans, and [`Text`] is lines. No escape sequence exists in any
//! of them: whatever writes the lines to a terminal adds the styling after
//! layout, so nothing here strips or repairs one.
//!
//! # Quick start
//!
//! Put text in a [`Text`], pick an [`Overflow`] policy, and render it at a
//! width. The default policy keeps each word whole, so a long URL keeps a line
//! of its own and runs past the width instead of breaking:
//!
//! ```
//! use ctl_text::anstyle::AnsiColor;
//! use ctl_text::{Context, Renderable, Style, Text};
//!
//! let green = Style::new().fg_color(Some(AnsiColor::Green.into()));
//! let text = Text::styled("see https://example.test/releases/tag/pkg@1.2.3", green);
//!
//! let lines = text.render(&Context::new(30));
//! assert_eq!(lines[0].to_string(), "see");
//! assert_eq!(
//!     lines[1].to_string(),
//!     "https://example.test/releases/tag/pkg@1.2.3"
//! );
//! ```
//!
//! # Safe by construction
//!
//! Text from outside the program, such as a commit subject, can hold an
//! escape sequence that moves the cursor or a directional override that
//! reorders the line. [`Span::new`] and [`Text`] remove every character that
//! [`verbatim::is_unsafe`] refuses, so a widget built on them cannot pass one
//! on.
//!
//! # Widths
//!
//! [`text_width`] counts terminal cells per extended grapheme cluster, so a
//! family emoji is 2 cells and `日本` is 4. The cluster boundaries and the
//! widths come from two Unicode data sets, whose versions
//! [`UNICODE_VERSION`] states.
//!
//! # Writing a widget
//!
//! A widget implements [`Renderable`]: it says how narrow and how wide it can
//! be, then draws itself at the width it is given. A container does the same
//! over its children.
//!
//! ```
//! use ctl_text::anstyle::Effects;
//! use ctl_text::{Context, Line, Measurement, Renderable, Span, Style};
//!
//! /// A bar of up to 12 cells, filled in proportion to `done / total`.
//! struct Meter {
//!     done: u16,
//!     total: u16,
//! }
//!
//! impl Renderable for Meter {
//!     fn measure(&self, _: &Context) -> Measurement {
//!         Measurement::exact(12)
//!     }
//!
//!     fn render(&self, context: &Context) -> Vec<Line> {
//!         let cells = context.width().min(12);
//!         let share = u32::from(cells) * u32::from(self.done.min(self.total))
//!             / u32::from(self.total.max(1));
//!         let filled = u16::try_from(share).unwrap_or(cells);
//!         let bar = "█".repeat(filled.into());
//!         vec![Line::from(Span::new(
//!             bar,
//!             Style::new().effects(Effects::BOLD),
//!         ))]
//!     }
//! }
//!
//! let meter = Meter { done: 3, total: 4 };
//! assert_eq!(meter.measure(&Context::new(80)), Measurement::exact(12));
//! assert_eq!(meter.render(&Context::new(80))[0].to_string(), "█████████");
//! ```

mod cells;
mod line;
mod protocol;
mod sanitize;
mod span;
mod style;
mod text;
pub mod verbatim;
mod wrap;

pub use anstyle::{self, Style};
pub use cells::{
    TAB_WIDTH, UNICODE_VERSION, UnicodeVersions, cluster_width, prefix_len, text_width,
};
pub use line::Line;
pub use protocol::{BoxChars, Context, Measurement, Renderable};
pub use span::Span;
pub use style::overlay;
pub use text::Text;
pub use wrap::{ELLIPSIS, Overflow};

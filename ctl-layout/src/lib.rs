//! Terminal widgets built on [`ctl_text`]: tables, records, rules, stacks,
//! indents, and a way to swap one widget for another by width.
//!
//! Every widget implements [`Renderable`]. It first *measures*: the narrowest
//! width it can be drawn at without breaking a word it keeps whole, and its
//! natural width. Then it *renders* lines at the width it is given.
//!
//! A container measures its children, decides their widths, and renders each
//! child at its own. The result is plain lines of styled spans with no escape
//! sequence in them. Whatever writes the lines to a terminal adds the styling.
//!
//! The crate re-exports [`ctl_text`] and the four types every widget uses:
//! [`Context`], [`Overflow`], [`Renderable`], and [`Text`].
//!
//! # Widgets
//!
//! - [`Table`]: rows under column headers. A [`Column`] sets the header, the
//!   overflow policy, and how its cells are aligned ([`Align`]). A [`Frame`]
//!   and a [`RowSeparation`] set the rules between them.
//! - [`Record`]: labelled values, drawn in a [`RecordStyle`].
//! - [`Definitions`]: entries as a line of labels over indented description
//!   lines, the narrow form of a table.
//! - [`Breakpoint`]: one widget below a width and another from it up.
//! - [`Rule`]: a horizontal line with an optional title.
//! - [`Stack`]: widgets one above the other.
//! - [`Indent`]: a widget moved right.
//!
//! # Quick start
//!
//! A [`Table`] gives each column the widest cell it holds, and shrinks the
//! widest columns first when the width runs short. A column sets what
//! happens to a cell that does not fit, and which edge its cells sit against:
//!
//! ```
//! use ctl_layout::{Column, Context, Frame, Overflow, Renderable, Table};
//!
//! let table = Table::new([
//!     Column::new("id"),
//!     Column::new("title").overflow(Overflow::Ellipsis),
//!     Column::new("age").numeric(),
//! ])
//! .frame(Frame::HeaderRule)
//! .row(["job-0014", "Fix the layout of narrow tables", "3"])
//! .row(["job-0015", "Document each public item", "120"]);
//!
//! let lines: Vec<String> = table
//!     .render(&Context::new(40))
//!     .iter()
//!     .map(ToString::to_string)
//!     .collect();
//! assert_eq!(
//!     lines,
//!     [
//!         "id        title                      age",
//!         "────────────────────────────────────────",
//!         "job-0014  Fix the layout of narrow…    3",
//!         "job-0015  Document each public item  120",
//!     ]
//! );
//! ```
//!
//! A [`Record`] draws labelled values, a [`Rule`] draws a line with an optional
//! title, a [`Stack`] puts widgets one above the other, and an [`Indent`] moves
//! one right. A stack and an indent take a widget by value or by reference:
//!
//! ```
//! use ctl_layout::{Context, Indent, Record, Renderable, Rule, Stack, Text};
//!
//! let screen = Stack::new()
//!     .spacing(1)
//!     .push(Rule::titled(Text::plain("Release")))
//!     .push(Indent::new(
//!         2,
//!         Record::new()
//!             .row("version", "1.4.0")
//!             .row("channel", "stable"),
//!     ));
//!
//! let lines: Vec<String> = screen
//!     .render(&Context::new(24))
//!     .iter()
//!     .map(ToString::to_string)
//!     .collect();
//! assert_eq!(
//!     lines,
//!     [
//!         "── Release ─────────────",
//!         "",
//!         "  version  1.4.0",
//!         "  channel  stable",
//!     ]
//! );
//! ```
//!
//! A [`Breakpoint`] draws [`Definitions`] where a table no longer fits, so a
//! narrow terminal gets one line of flags over an indented description:
//!
//! ```
//! use ctl_layout::{Breakpoint, Column, Context, Definitions, Frame, Renderable, Table};
//!
//! let flags = Breakpoint::new(
//!     40,
//!     Definitions::new()
//!         .entry(["-v", "--verbose"], ["Print more detail"])
//!         .entry(["-q", "--quiet"], ["Print nothing on success"]),
//!     Table::new([Column::untitled(), Column::untitled(), Column::untitled()])
//!         .frame(Frame::Plain)
//!         .row(["-v", "--verbose", "Print more detail"])
//!         .row(["-q", "--quiet", "Print nothing on success"]),
//! );
//!
//! let at = |width| -> Vec<String> {
//!     flags
//!         .render(&Context::new(width))
//!         .iter()
//!         .map(ToString::to_string)
//!         .collect()
//! };
//! assert_eq!(
//!     at(50),
//!     [
//!         "-v  --verbose  Print more detail",
//!         "-q  --quiet    Print nothing on success",
//!     ]
//! );
//! assert_eq!(
//!     at(30),
//!     [
//!         "  -v --verbose",
//!         "    Print more detail",
//!         "  -q --quiet",
//!         "    Print nothing on success",
//!     ]
//! );
//! ```
//!
//! A widget of your own sits in a [`Stack`] beside these once it implements
//! [`Renderable`]. Its [`Measurement`](ctl_text::Measurement) and
//! [`Line`](ctl_text::Line) types come through
//! [`ctl_layout::ctl_text`](ctl_text).
//!
//! # Features
//!
//! This crate has no optional features.

mod align;
mod breakpoint;
mod column;
mod definitions;
mod frame;
mod glyphs;
mod indent;
mod lines;
mod record;
mod rule;
mod stack;
mod table;
mod widths;

pub use align::Align;
pub use breakpoint::Breakpoint;
pub use column::Column;
pub use ctl_text::{self, Context, Overflow, Renderable, Text};
pub use definitions::Definitions;
pub use frame::{Frame, RecordStyle, RowSeparation};
pub use indent::Indent;
pub use record::Record;
pub use rule::Rule;
pub use stack::Stack;
pub use table::Table;

/// The README's examples, compiled and run as doctests.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

//! Terminal widgets built on [`ctl_text`]: tables, records, rules, stacks, and
//! indents.
//!
//! Every widget implements [`Renderable`](ctl_text::Renderable). It first
//! *measures*: the narrowest width it can be drawn at without breaking a word
//! it keeps whole, and its natural width. Then it *renders* lines at the width
//! it is given.
//!
//! A container measures its children, decides their widths, and renders each
//! child at its own. The result is plain lines of styled spans with no escape
//! sequence in them. Whatever writes the lines to a terminal adds the styling.
//!
//! # Widgets
//!
//! - [`Table`]: rows under column headers. A [`Column`] sets the header, the
//!   overflow policy, and how its cells are aligned ([`Align`]). A [`Frame`]
//!   and a [`RowSeparation`] set the rules between them.
//! - [`Record`]: labelled values, drawn in a [`RecordStyle`].
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
//! use ctl_layout::{Column, Frame, Table};
//! use ctl_text::{Context, Overflow, Renderable, Text};
//!
//! let table = Table::new([
//!     Column::new("id"),
//!     Column::new("title").overflow(Overflow::Ellipsis),
//!     Column::new("age").numeric(),
//! ])
//! .frame(Frame::HeaderRule)
//! .row([Text::plain("job-0014"), Text::plain("Fix the layout of narrow tables"), Text::plain("3")])
//! .row([Text::plain("job-0015"), Text::plain("Document each public item"), Text::plain("120")]);
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
//! one right:
//!
//! ```
//! use ctl_layout::{Indent, Record, Rule, Stack};
//! use ctl_text::{Context, Renderable, Text};
//!
//! let screen = Stack::new()
//!     .spacing(1)
//!     .push(Rule::titled(Text::plain("Ledger")))
//!     .push(Indent::new(
//!         2,
//!         Record::new()
//!             .row(Text::plain("ledger"), Text::plain("tasks.yaml"))
//!             .row(Text::plain("active"), Text::plain("job-0014")),
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
//!         "── Ledger ──────────────",
//!         "",
//!         "  ledger  tasks.yaml",
//!         "  active  job-0014",
//!     ]
//! );
//! ```
//!
//! A widget of your own sits in a [`Stack`] beside these once it implements
//! [`Renderable`](ctl_text::Renderable).

mod align;
mod column;
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
pub use column::Column;
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

use std::io::{self, Write as _};

use crate::format::tabular::{self, Grid, Part};
use crate::format::{Format, FormatName, Output, Sink, TSV_ABOUT};

/// The `tsv` format: the result's document as tab-separated lines.
///
/// A table is its header line, when it has headers, then one line per row,
/// each cell joined by a tab. Fields are one `label<TAB>value` line per row,
/// with no header. A paragraph and verbatim text are written as they are,
/// with trailing line breaks dropped.
///
/// A notice is `level<TAB>message`, the level being `success`, `warning`, or
/// `error`. A code goes before the message as `code: message`. Headings,
/// section titles, and rules are left out. A section writes its body in place.
///
/// A tab, line feed, carriage return, or backslash inside a cell, label,
/// value, or notice is written as `\t`, `\n`, `\r`, or `\\`. A record then
/// stays on one line.
///
/// Styling is dropped. Blocks that write something are separated by one blank
/// line, every line ends in a line feed, and an empty document writes nothing.
///
/// It is a machine format, so it never carries ANSI, and it is the default
/// when stdout is not a terminal. Its error form is the default,
/// `{bin}: {message}`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tsv;

/// The built-in `tsv` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static TSV: Tsv = Tsv;

/// `cell` with the four characters that would end or split a record written as
/// escapes.
fn escape(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    for character in cell.chars() {
        match character {
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}

/// One line: the escaped `cells` joined by tabs.
fn line(cells: &[String]) -> String {
    let cells: Vec<String> = cells.iter().map(|cell| escape(cell)).collect();
    let mut line = cells.join("\t");
    line.push('\n');
    line
}

/// Text as it is, ending in one line feed, or nothing when it is empty.
fn text(text: &str) -> String {
    let text = tabular::trimmed(text);
    if text.is_empty() {
        String::new()
    } else {
        format!("{text}\n")
    }
}

fn chunk(part: &Part<'_>) -> String {
    match part {
        Part::Heading { .. } | Part::Rule(_) => String::new(),
        Part::Paragraph(paragraph) => text(paragraph),
        Part::Verbatim(verbatim) => text(verbatim),
        Part::Fields(fields) => tabular::records(fields)
            .iter()
            .map(|record| line(record))
            .collect(),
        Part::Table(table) => Grid::of(table).lines().map(|row| line(row)).collect(),
        Part::Notice(notice) => {
            let message = tabular::plain(notice.message());
            let message = match notice.code_value() {
                Some(code) => format!("{code}: {message}"),
                None => message,
            };
            line(&[tabular::level_name(notice.level()).to_owned(), message])
        }
    }
}

impl Format for Tsv {
    fn name(&self) -> FormatName {
        FormatName::TSV
    }

    fn about(&self) -> &'static str {
        TSV_ABOUT
    }

    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        let parts = tabular::parts(output.document());
        let written = tabular::join(parts.iter().map(chunk), "\n");
        sink.write_all(written.as_bytes())
    }
}

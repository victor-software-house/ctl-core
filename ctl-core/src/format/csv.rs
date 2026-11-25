use std::io::{self, Write as _};

use crate::format::tabular::{self, Buffer, Dialect, Place, Source};
use crate::format::{CSV_ABOUT, Format, FormatName, Output, Sink};

/// The `csv` format: the result's tables and fields as comma-separated values,
/// as RFC 4180 writes them.
///
/// A table is a header record, when it has headers, then one record per row.
/// Fields are a `field,value` header record, then one record per row.
/// Paragraphs, verbatim text, notices, headings, section titles, and rules
/// are left out, and a section writes its body in place. Styling is dropped.
///
/// A field with a comma, a double quote, a carriage return, or a line feed is
/// quoted, and each quote inside it is doubled. A record of one empty field
/// is written as `""`, so it is not a blank line. Blocks that write something
/// are separated by one blank line. Every record and blank line ends in CRLF.
/// A document with no table or fields writes nothing.
///
/// Every control character, directional formatting character, and line or
/// paragraph separator but a tab, line feed, and carriage return is removed
/// from a field, so a value cannot carry an escape sequence to the terminal
/// that shows it.
///
/// Values are written as they are. A spreadsheet runs a cell that starts with
/// `=`, `+`, `-`, `@`, a tab, or a carriage return as a formula when it opens
/// the file, and `csv` does not prefix such a cell, because the prefix would
/// change the data every other reader gets. A CLI that writes CSV for a
/// spreadsheet registers a format of its own, under the name `csv` or
/// another, that guards those cells.
///
/// It is a machine format, so it writes no ANSI styling. Its error form is the
/// default, `{bin}: {message}`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Csv;

/// The built-in `csv` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static CSV: Csv = Csv;

impl Dialect for Csv {
    fn eol(&self) -> &'static str {
        "\r\n"
    }

    fn cell(&self, out: &mut Buffer, at: Place, text: Source<'_>) {
        if at.index > 0 {
            out.push(',');
        }
        let quoted = text.chars().any(|c| matches!(c, ',' | '"' | '\r' | '\n'))
            || (at.count == 1 && text.is_empty());
        if quoted {
            out.push('"');
        }
        for character in text.chars() {
            if character == '"' {
                out.push('"');
            }
            out.push(character);
        }
        if quoted {
            out.push('"');
        }
    }

    fn end_record(&self, out: &mut Buffer) {
        out.push_str("\r\n");
    }

    fn fields_header(&self) -> bool {
        true
    }
}

impl Format for Csv {
    fn name(&self) -> FormatName {
        FormatName::CSV
    }

    fn about(&self) -> &'static str {
        CSV_ABOUT
    }

    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        let written = tabular::write(output.document(), self);
        sink.write_all(written.as_bytes())
    }
}

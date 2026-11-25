use std::io::{self, Write as _};

use crate::format::tabular::{self, Grid, Part};
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
/// quoted, and each quote inside it is doubled. Blocks that write something
/// are separated by one blank line. Every record and blank line ends in CRLF.
/// A document with no table or fields writes nothing.
///
/// It is a machine format, so it never carries ANSI. Its error form is the
/// default, `{bin}: {message}`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Csv;

/// The built-in `csv` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static CSV: Csv = Csv;

/// The end of every record and of the blank line between blocks.
const EOL: &str = "\r\n";

/// One record: the `cells`, each quoted when it needs it, ending in CRLF.
fn record(cells: &[String]) -> String {
    let mut record = String::new();
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            record.push(',');
        }
        if cell.contains([',', '"', '\r', '\n']) {
            record.push('"');
            record.push_str(&cell.replace('"', "\"\""));
            record.push('"');
        } else {
            record.push_str(cell);
        }
    }
    record.push_str(EOL);
    record
}

fn chunk(part: &Part<'_>) -> String {
    match part {
        Part::Heading { .. }
        | Part::Paragraph(_)
        | Part::Verbatim(_)
        | Part::Notice(_)
        | Part::Rule(_) => String::new(),
        Part::Fields(fields) => {
            let header = ["field".to_owned(), "value".to_owned()];
            let records = tabular::records(fields);
            [header.as_slice()]
                .into_iter()
                .chain(records.iter().map(Vec::as_slice))
                .map(record)
                .collect()
        }
        Part::Table(table) => Grid::of(table).lines().map(|row| record(row)).collect(),
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
        let parts = tabular::parts(output.document());
        let written = tabular::join(parts.iter().map(chunk), EOL);
        sink.write_all(written.as_bytes())
    }
}

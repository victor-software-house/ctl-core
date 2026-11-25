use std::io::{self, Write as _};

use crate::document::Notice;
use crate::format::tabular::{self, Buffer, Dialect, Place, Source};
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
/// A record of one empty cell is a blank line, which a reader takes for the
/// end of a block. `tsv` has no escape for it, so a table with one column
/// should not have an empty value.
///
/// Styling is dropped, and so is every control character, directional
/// formatting character, and line or paragraph separator but a tab, line
/// feed, and carriage return, so a value cannot carry an escape sequence to
/// the terminal that shows it. Blocks that write something are separated by
/// one blank line, every line ends in a line feed, and an empty document
/// writes nothing.
///
/// It is a machine format, so it never writes ANSI styling, and it is the
/// default when stdout is not a terminal. Its error form is the default,
/// `{bin}: {message}`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tsv;

/// The built-in `tsv` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static TSV: Tsv = Tsv;

/// `text` with the four characters that would end or split a record written
/// as escapes.
fn escape(out: &mut Buffer, text: Source<'_>) {
    for character in text.chars() {
        match character {
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
}

impl Dialect for Tsv {
    fn eol(&self) -> &'static str {
        "\n"
    }

    fn cell(&self, out: &mut Buffer, at: Place, text: Source<'_>) {
        if at.index > 0 {
            out.push('\t');
        }
        escape(out, text);
    }

    fn end_record(&self, out: &mut Buffer) {
        out.push('\n');
    }

    fn paragraph(&self, out: &mut Buffer, text: Source<'_>) {
        tabular::lines(out, text);
    }

    fn verbatim(&self, out: &mut Buffer, text: Source<'_>) {
        tabular::lines(out, text);
    }

    fn notice(&self, out: &mut Buffer, notice: &Notice) {
        out.push_str(tabular::level_name(notice.level()));
        out.push('\t');
        if let Some(code) = notice.code_value() {
            escape(out, Source::Str(code));
            out.push_str(": ");
        }
        escape(out, Source::Text(notice.message()));
        out.push('\n');
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
        let written = tabular::write(output.document(), self);
        sink.write_all(written.as_bytes())
    }
}

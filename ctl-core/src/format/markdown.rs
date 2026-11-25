use std::io::{self, Write as _};
use std::mem;

use crate::document::Notice;
use crate::format::tabular::{self, Buffer, Dialect, Place, Source};
use crate::format::{Format, FormatName, MARKDOWN_ABOUT, Output, RecordWriter, Sink};

/// The `markdown` format: the result's document as Markdown. Every block is
/// written, and styling is dropped.
///
/// A table is a pipe table: a header row, a `| --- |` separator row, then one
/// row per row. A table with no headers gets an empty header row. Fields are
/// a two-column pipe table headed `field` and `value`.
///
/// In a table cell, a backslash is written as `\\`, `|` as `\|`, `&` as
/// `&amp;`, `<` as `&lt;`, and a line break as `<br>`, so a cell that holds
/// `<br>` or `\|` itself stays distinguishable from the escapes. No other
/// Markdown character is escaped, so a cell that holds `*` or a backtick is
/// written as it is.
///
/// A paragraph is its text. Verbatim text is a fenced code block, with a fence
/// longer than any run of backticks in it. A notice is a quoted line,
/// `> **level**: message`, with `(code)` after the level when it has one.
///
/// A heading is a heading at the enclosing level: level 1 at the top, one
/// deeper inside each section, at most 6. A section writes its title as a
/// heading, then its body one level deeper. A heading, a section title, and
/// a rule title are one line, each line break in them a space, and one that is
/// empty after trimming is not written. A rule is `---`, preceded by
/// `**title**` and a blank line when it has a title, so Markdown reads a
/// paragraph and a rule, not a heading.
///
/// Every control character, directional formatting character, and line or
/// paragraph separator but a tab, line feed, and carriage return is removed
/// from the text, so a value cannot carry an escape sequence to the terminal
/// that shows it.
///
/// Blocks are separated by one blank line, every line ends in a line feed, and
/// an empty document writes nothing.
///
/// Under `--stream`, each record is written as it arrives, as the whole
/// output writes its document, with the same blank line between blocks. A
/// table with the header row and column count of the last table written goes
/// on with only its rows, and no blank line before them. A plain table goes
/// on from the last plain table of the same column count. So records that are
/// each one row of one table are, byte for byte, the collected output. Any
/// other table writes its header row and separator row again, after a blank
/// line. Every other block, such as fields, a paragraph, a heading, or a rule,
/// ends the table, so the same table after one of them writes its header
/// again. A heading that is empty after trimming writes nothing and does not
/// end the table.
///
/// It is a machine format, so it writes no ANSI styling. Its error form is the
/// default, `{bin}: {message}`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Markdown;

/// The built-in `markdown` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static MARKDOWN: Markdown = Markdown;

/// The fewest backticks a fence has.
const SHORTEST_FENCE: usize = 3;

/// `text` as the inside of a table cell. A carriage return and line feed
/// together are one line break.
fn escape(out: &mut Buffer, text: Source<'_>) {
    let mut after_return = false;
    for character in text.chars() {
        let was_return = mem::replace(&mut after_return, character == '\r');
        match character {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '\n' if was_return => {}
            '\n' | '\r' => out.push_str("<br>"),
            other => out.push(other),
        }
    }
}

/// `text` in a fence longer than any run of backticks in it.
fn fenced(out: &mut Buffer, text: Source<'_>) {
    let length = text.trimmed_length();
    if length == 0 {
        return;
    }
    let mut run = 0;
    let mut longest = 0;
    for character in text.chars().take(length) {
        run = if character == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = (longest + 1).max(SHORTEST_FENCE);
    out.repeat('`', fence);
    out.push('\n');
    out.push_all(text.chars().take(length));
    out.push('\n');
    out.repeat('`', fence);
    out.push('\n');
}

impl Dialect for Markdown {
    fn eol(&self) -> &'static str {
        "\n"
    }

    fn cell(&self, out: &mut Buffer, at: Place, text: Source<'_>) {
        if at.index == 0 {
            out.push('|');
        }
        out.push(' ');
        escape(out, text);
        out.push_str(" |");
    }

    fn end_record(&self, out: &mut Buffer) {
        out.push('\n');
    }

    fn blank_header(&self) -> bool {
        true
    }

    fn fields_header(&self) -> bool {
        true
    }

    fn after_header(&self, out: &mut Buffer, width: usize) {
        for index in 0..width {
            self.cell(
                out,
                Place {
                    index,
                    count: width,
                },
                Source::Str("---"),
            );
        }
        self.end_record(out);
    }

    fn paragraph(&self, out: &mut Buffer, text: Source<'_>) {
        tabular::lines(out, text);
    }

    fn verbatim(&self, out: &mut Buffer, text: Source<'_>) {
        fenced(out, text);
    }

    fn heading(&self, out: &mut Buffer, level: usize, title: Source<'_>) {
        let Some(title) = tabular::inline(title) else {
            return;
        };
        out.repeat('#', level);
        out.push(' ');
        out.push_all(title);
        out.push('\n');
    }

    fn notice(&self, out: &mut Buffer, notice: &Notice) {
        out.push_str("> **");
        out.push_str(tabular::level_name(notice.level()));
        out.push_str("**");
        if let Some(code) = notice.code_value() {
            out.push_str(" (");
            if let Some(code) = tabular::inline(Source::Str(code)) {
                out.push_all(code);
            }
            out.push(')');
        }
        out.push_str(": ");
        let message = Source::Text(notice.message());
        let mut characters = message.chars().take(message.trimmed_length()).peekable();
        while let Some(character) = characters.next() {
            match character {
                '\r' if characters.peek() == Some(&'\n') => {}
                '\n' | '\r' => out.push_str("\n> "),
                other => out.push(other),
            }
        }
        out.push('\n');
    }

    fn rule(&self, out: &mut Buffer, title: Option<Source<'_>>) {
        if let Some(title) = title.and_then(tabular::inline) {
            out.push_str("**");
            out.push_all(title);
            out.push_str("**\n\n");
        }
        out.push_str("---\n");
    }
}

impl Format for Markdown {
    fn name(&self) -> FormatName {
        FormatName::MARKDOWN
    }

    fn about(&self) -> &'static str {
        MARKDOWN_ABOUT
    }

    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        let written = tabular::write(output.document(), self);
        sink.write_all(written.as_bytes())
    }

    fn stream<'s>(&'s self, sink: Sink<'s>) -> Box<dyn RecordWriter + 's> {
        tabular::stream(self, sink)
    }
}

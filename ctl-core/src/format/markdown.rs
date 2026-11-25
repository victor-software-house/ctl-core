use std::io::{self, Write as _};

use crate::document::Notice;
use crate::format::tabular::{self, Grid, Part};
use crate::format::{Format, FormatName, MARKDOWN_ABOUT, Output, Sink};

/// The `markdown` format: the result's document as Markdown. Every block is
/// written, and styling is dropped.
///
/// A table is a pipe table: a header row, a `| --- |` separator row, then one
/// row per row. A table with no headers gets an empty header row. Fields are
/// a two-column pipe table headed `field` and `value`. In a cell, `|` is
/// written as `\|` and a line break as `<br>`.
///
/// A paragraph is its text. Verbatim text is a fenced code block, with a fence
/// longer than any run of backticks in it. A notice is a quoted line,
/// `> **level**: message`, with `(code)` after the level when it has one.
///
/// A heading is a heading at the enclosing level: level 1 at the top, one
/// deeper inside each section, at most 6. A section writes its title as a
/// heading, then its body one level deeper. A rule is `---`, preceded by
/// `**title**` on its own line when it has a title.
///
/// Blocks are separated by one blank line, every line ends in a line feed, and
/// an empty document writes nothing.
///
/// It is a machine format, so it never carries ANSI. Its error form is the
/// default, `{bin}: {message}`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Markdown;

/// The built-in `markdown` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static MARKDOWN: Markdown = Markdown;

/// `cell` as the inside of a table cell.
fn escape(cell: &str) -> String {
    cell.replace('|', "\\|")
        .replace("\r\n", "<br>")
        .replace(['\r', '\n'], "<br>")
}

/// One table row of `cells`.
fn row(cells: &[String]) -> String {
    let mut row = String::from("|");
    for cell in cells {
        row.push(' ');
        row.push_str(&escape(cell));
        row.push_str(" |");
    }
    row.push('\n');
    row
}

/// A pipe table with `header` above `rows`.
fn table(header: &[String], rows: &[Vec<String>]) -> String {
    let mut table = row(header);
    table.push_str(&row(&vec!["---".to_owned(); header.len()]));
    table.extend(rows.iter().map(|cells| row(cells)));
    table
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

/// `text` in a fence longer than any run of backticks in it.
fn fenced(text: &str) -> String {
    let text = tabular::trimmed(text);
    if text.is_empty() {
        return String::new();
    }
    let longest = text
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or_default();
    let fence = "`".repeat((longest + 1).max(3));
    format!("{fence}\n{text}\n{fence}\n")
}

/// A quoted notice: every line after the first is quoted too.
fn notice(notice: &Notice) -> String {
    let level = tabular::level_name(notice.level());
    let code = notice
        .code_value()
        .map_or_else(String::new, |code| format!(" ({code})"));
    let message = tabular::plain(notice.message());
    let mut lines = tabular::trimmed(&message).lines();
    let first = lines.next().unwrap_or_default();
    let mut quoted = format!("> **{level}**{code}: {first}\n");
    for rest in lines {
        quoted.push_str("> ");
        quoted.push_str(rest);
        quoted.push('\n');
    }
    quoted
}

fn chunk(part: &Part<'_>) -> String {
    match part {
        Part::Heading { level, text } => format!("{} {text}\n", "#".repeat(*level)),
        Part::Paragraph(paragraph) => text(paragraph),
        Part::Verbatim(verbatim) => fenced(verbatim),
        Part::Fields(fields) => table(
            &["field".to_owned(), "value".to_owned()],
            &tabular::records(fields),
        ),
        Part::Table(cells) => {
            let grid = Grid::of(cells);
            if grid.is_empty() {
                String::new()
            } else {
                table(&grid.header_or_blank(), grid.rows())
            }
        }
        Part::Notice(quoted) => notice(quoted),
        Part::Rule(None) => "---\n".to_owned(),
        Part::Rule(Some(title)) => format!("**{title}**\n---\n"),
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
        let parts = tabular::parts(output.document());
        let written = tabular::join(parts.iter().map(chunk), "\n");
        sink.write_all(written.as_bytes())
    }
}

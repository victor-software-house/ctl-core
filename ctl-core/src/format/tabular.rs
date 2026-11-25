//! What `tsv`, `csv`, and `markdown` share: one walk over a document that
//! writes into one `String`, through the [`Dialect`] each format implements,
//! and the one writer of their stream form.

use std::io::{self, Write as _};
use std::{mem, str};

use crate::document::{Block, Document, Fields, Notice, NoticeLevel, Span, Table, Text};
use crate::format::{Output, RecordWriter, Sink};
use crate::unsafe_chars::is_unsafe;

/// The deepest heading level Markdown has.
const DEEPEST_HEADING: usize = 6;

/// The label and value headings of a fields block, in a format that has one.
const FIELDS_HEADER: [&str; 2] = ["field", "value"];

/// Where a cell is in its record.
#[derive(Clone, Copy)]
pub(super) struct Place {
    /// The cell's position in the record, from 0.
    pub(super) index: usize,
    /// The number of cells in the record.
    pub(super) count: usize,
}

/// A format's rules for what the walk writes. The walk is the same for every
/// format, so it compiles once, and each format answers only where it
/// differs.
pub(super) trait Dialect {
    /// The line break of a blank line between two blocks.
    fn eol(&self) -> &'static str;

    /// One cell of a record, escaped, with the separator before it when it
    /// is not the first.
    fn cell(&self, out: &mut Buffer, at: Place, text: Source<'_>);

    /// The end of a record.
    fn end_record(&self, out: &mut Buffer);

    /// Whether a table with no headers still writes an empty header row.
    fn blank_header(&self) -> bool {
        false
    }

    /// Whether a fields block starts with a `field` and `value` header.
    fn fields_header(&self) -> bool {
        false
    }

    /// What follows a header row of `width` cells.
    fn after_header(&self, _out: &mut Buffer, _width: usize) {}

    /// A paragraph.
    fn paragraph(&self, _out: &mut Buffer, _text: Source<'_>) {}

    /// Verbatim text.
    fn verbatim(&self, _out: &mut Buffer, _text: Source<'_>) {}

    /// A heading, or a section title, at `level` (1 to 6).
    fn heading(&self, _out: &mut Buffer, _level: usize, _title: Source<'_>) {}

    /// A notice.
    fn notice(&self, _out: &mut Buffer, _notice: &Notice) {}

    /// A rule, with its title when it has one.
    fn rule(&self, _out: &mut Buffer, _title: Option<Source<'_>>) {}
}

/// The header row and width of the table last written.
struct Header {
    cells: Vec<Text>,
    width: usize,
}

/// What a stream keeps from one record to the next: the header of the table
/// it last wrote, or none, and whether it wrote anything.
#[derive(Default)]
struct Run {
    last: Option<Header>,
    written: bool,
}

/// The output so far, and the blank line owed before the next block's first
/// byte. A block that writes nothing leaves the debt unpaid, so it adds no
/// blank line.
pub(super) struct Buffer {
    text: String,
    blank: &'static str,
    owed: bool,
    /// Whether a table can continue the last one written. Only a stream's
    /// buffer sets it; the whole output writes every table's header.
    stream: bool,
    run: Run,
}

impl Buffer {
    fn new(blank: &'static str) -> Self {
        Self {
            text: String::new(),
            blank,
            owed: false,
            stream: false,
            run: Run::default(),
        }
    }

    /// A buffer for one record of a stream, which carries `run` over from the
    /// records before it.
    fn resume(blank: &'static str, run: Run) -> Self {
        Self {
            stream: true,
            run,
            ..Self::new(blank)
        }
    }

    /// Starts a block: it owes a blank line when something was written, here
    /// or in an earlier record of the stream.
    fn block(&mut self) {
        self.owed = self.run.written || !self.text.is_empty();
    }

    /// Ends the current table when the block that began at byte `start` wrote
    /// something.
    fn end_table_if_written(&mut self, start: usize) {
        if self.text.len() > start {
            self.run.last = None;
        }
    }

    /// Whether `table`, `width` cells wide, continues the table last written.
    fn continues(&self, table: &Table, width: usize) -> bool {
        self.stream
            && self
                .run
                .last
                .as_ref()
                .is_some_and(|last| last.width == width && last.cells == table.headers())
    }

    fn pay(&mut self) {
        if mem::take(&mut self.owed) {
            self.text.push_str(self.blank);
        }
    }

    pub(super) fn push(&mut self, character: char) {
        self.pay();
        self.text.push(character);
    }

    pub(super) fn push_str(&mut self, text: &str) {
        self.pay();
        self.text.push_str(text);
    }

    /// Writes `character` `times` times.
    pub(super) fn repeat(&mut self, character: char, times: usize) {
        for _ in 0..times {
            self.push(character);
        }
    }

    pub(super) fn push_all(&mut self, characters: impl Iterator<Item = char>) {
        for character in characters {
            self.push(character);
        }
    }
}

/// The text of a cell or a block: a [`Text`] or a string, read as its
/// characters.
#[derive(Clone, Copy)]
pub(super) enum Source<'a> {
    Text(&'a Text),
    Str(&'a str),
}

impl<'a> Source<'a> {
    /// The characters, styling dropped. A control, a directional formatting
    /// character, and a line or paragraph separator are dropped too, except
    /// a tab, line feed, and carriage return, which each format writes in its
    /// own way.
    pub(super) fn chars(self) -> Chars<'a> {
        match self {
            Self::Text(text) => Chars {
                rest: text.spans(),
                current: "".chars(),
            },
            Self::Str(text) => Chars {
                rest: &[],
                current: text.chars(),
            },
        }
    }

    pub(super) fn is_empty(self) -> bool {
        self.chars().next().is_none()
    }

    /// How many characters come before the trailing line breaks.
    pub(super) fn trimmed_length(self) -> usize {
        self.chars().enumerate().fold(0, |end, (index, character)| {
            if matches!(character, '\n' | '\r') {
                end
            } else {
                index + 1
            }
        })
    }
}

/// The characters of a [`Source`].
#[derive(Clone)]
pub(super) struct Chars<'a> {
    rest: &'a [Span],
    current: str::Chars<'a>,
}

impl Iterator for Chars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        loop {
            match self.current.next() {
                Some(character)
                    if matches!(character, '\t' | '\n' | '\r') || !is_unsafe(character) =>
                {
                    return Some(character);
                }
                Some(_) => {}
                None => {
                    let (first, rest) = self.rest.split_first()?;
                    self.current = first.value().chars();
                    self.rest = rest;
                }
            }
        }
    }
}

/// The lowercase name of a notice's level.
pub(super) const fn level_name(level: NoticeLevel) -> &'static str {
    match level {
        NoticeLevel::Success => "success",
        NoticeLevel::Warning => "warning",
        NoticeLevel::Error => "error",
    }
}

/// `text` as it is, without its trailing line breaks, and one line feed. A
/// text that is only line breaks writes nothing.
pub(super) fn lines(out: &mut Buffer, text: Source<'_>) {
    let length = text.trimmed_length();
    if length > 0 {
        out.push_all(text.chars().take(length));
        out.push('\n');
    }
}

/// `text` on one line: trimmed, each line break a space. `None` when
/// nothing is left.
pub(super) fn inline(text: Source<'_>) -> Option<impl Iterator<Item = char> + '_> {
    let start = text.chars().take_while(|c| c.is_whitespace()).count();
    let end = text.chars().enumerate().fold(0, |end, (index, character)| {
        if character.is_whitespace() {
            end
        } else {
            index + 1
        }
    });
    (start < end).then(move || {
        let mut after_return = false;
        text.chars()
            .skip(start)
            .take(end - start)
            .filter_map(move |character| {
                let was_return = mem::replace(&mut after_return, character == '\r');
                match character {
                    '\n' if was_return => None,
                    '\n' | '\r' => Some(' '),
                    other => Some(other),
                }
            })
    })
}

/// Writes `document` as `dialect` says: every block in order, a section's
/// body in place, one blank line between blocks that wrote something.
pub(super) fn write(document: &Document, dialect: &dyn Dialect) -> String {
    let mut out = Buffer::new(dialect.eol());
    blocks(document, 1, dialect, &mut out);
    out.text
}

/// The stream form of `tsv`, `csv`, and `markdown`: the writer that
/// [`Format::stream`](crate::format::Format::stream) returns for each, over
/// the format's [`Dialect`].
pub(super) fn stream<'s>(dialect: &'s dyn Dialect, sink: Sink<'s>) -> Box<dyn RecordWriter + 's> {
    Box::new(Stream {
        dialect,
        sink,
        run: Run::default(),
    })
}

/// Writes each record's document as the whole output writes it, and keeps the
/// header of the last table across records, so a table that goes on is only
/// its rows.
struct Stream<'s> {
    dialect: &'s dyn Dialect,
    sink: Sink<'s>,
    run: Run,
}

impl RecordWriter for Stream<'_> {
    fn record(&mut self, output: &Output<'_>) -> io::Result<()> {
        let mut out = Buffer::resume(self.dialect.eol(), mem::take(&mut self.run));
        blocks(output.document(), 1, self.dialect, &mut out);
        self.run = out.run;
        self.run.written |= !out.text.is_empty();
        self.sink.write_all(out.text.as_bytes())
    }

    fn finish(self: Box<Self>) -> io::Result<()> {
        Ok(())
    }
}

fn blocks(document: &Document, level: usize, dialect: &dyn Dialect, out: &mut Buffer) {
    let heading = level.min(DEEPEST_HEADING);
    for block in document.blocks() {
        out.block();
        let start = out.text.len();
        match block {
            Block::Heading(text) => dialect.heading(out, heading, Source::Text(text)),
            Block::Paragraph(text) => dialect.paragraph(out, Source::Text(text)),
            Block::Verbatim(text) => dialect.verbatim(out, Source::Str(text)),
            Block::Fields(fields) => write_fields(fields, dialect, out),
            Block::Table(table) => write_table(table, dialect, out),
            Block::Section(section) => {
                dialect.heading(out, heading, Source::Text(section.title()));
                out.end_table_if_written(start);
                blocks(section.body(), level + 1, dialect, out);
            }
            Block::Notice(notice) => dialect.notice(out, notice),
            Block::Rule(rule) => dialect.rule(out, rule.title().map(Source::Text)),
        }
        if !matches!(block, Block::Table(_) | Block::Section(_)) {
            out.end_table_if_written(start);
        }
    }
}

/// One record of `cells`, padded to `width` with empty cells.
fn record(dialect: &dyn Dialect, out: &mut Buffer, cells: &[Text], width: usize) {
    for (index, cell) in cells.iter().enumerate() {
        dialect.cell(
            out,
            Place {
                index,
                count: width,
            },
            Source::Text(cell),
        );
    }
    for index in cells.len()..width {
        dialect.cell(
            out,
            Place {
                index,
                count: width,
            },
            Source::Str(""),
        );
    }
    dialect.end_record(out);
}

/// One record of two cells.
fn pair(dialect: &dyn Dialect, out: &mut Buffer, left: Source<'_>, right: Source<'_>) {
    dialect.cell(out, Place { index: 0, count: 2 }, left);
    dialect.cell(out, Place { index: 1, count: 2 }, right);
    dialect.end_record(out);
}

fn write_fields(fields: &Fields, dialect: &dyn Dialect, out: &mut Buffer) {
    if dialect.fields_header() {
        let [label, value] = FIELDS_HEADER;
        pair(dialect, out, Source::Str(label), Source::Str(value));
        dialect.after_header(out, 2);
    }
    for (label, value) in fields.rows() {
        pair(dialect, out, Source::Text(label), Source::Text(value));
    }
}

/// A table is as wide as its longest row or its header row, and a table with
/// no columns writes nothing. In a stream, a table with the header and width
/// of the last one written is only its rows, with no blank line before them.
fn write_table(table: &Table, dialect: &dyn Dialect, out: &mut Buffer) {
    let width = table
        .rows()
        .iter()
        .map(Vec::len)
        .chain([table.headers().len()])
        .max()
        .unwrap_or_default();
    if width == 0 {
        return;
    }
    if out.continues(table, width) {
        out.owed = false;
    } else {
        if !table.headers().is_empty() || dialect.blank_header() {
            record(dialect, out, table.headers(), width);
            dialect.after_header(out, width);
        }
        if out.stream {
            out.run.last = Some(Header {
                cells: table.headers().to_vec(),
                width,
            });
        }
    }
    for row in table.rows() {
        record(dialect, out, row, width);
    }
}

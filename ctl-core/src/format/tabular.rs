//! What `tsv`, `csv`, and `markdown` share: reading a document as a flat
//! list of parts, and the cells of a table or a record.

use crate::document::{Block, Document, Fields, Notice, NoticeLevel, Span, Table, Text};

/// One block of a document as a tabular format reads it. A section is
/// flattened: its title is a heading one level down, followed by its body.
pub(super) enum Part<'a> {
    /// A heading, or a section title, at `level` (1 to 6).
    Heading {
        level: usize,
        text: String,
    },
    Paragraph(String),
    Verbatim(&'a str),
    Fields(&'a Fields),
    Table(&'a Table),
    Notice(&'a Notice),
    Rule(Option<String>),
}

/// The deepest heading level Markdown has.
const DEEPEST_HEADING: usize = 6;

/// The parts of `document`, in order, with each section's body in place.
pub(super) fn parts(document: &Document) -> Vec<Part<'_>> {
    let mut parts = Vec::new();
    collect(document, 1, &mut parts);
    parts
}

fn collect<'a>(document: &'a Document, level: usize, parts: &mut Vec<Part<'a>>) {
    for block in document.blocks() {
        match block {
            Block::Heading(text) => parts.push(Part::Heading {
                level: level.min(DEEPEST_HEADING),
                text: plain(text),
            }),
            Block::Paragraph(text) => parts.push(Part::Paragraph(plain(text))),
            Block::Verbatim(text) => parts.push(Part::Verbatim(text)),
            Block::Fields(fields) => parts.push(Part::Fields(fields)),
            Block::Table(table) => parts.push(Part::Table(table)),
            Block::Section(section) => {
                parts.push(Part::Heading {
                    level: level.min(DEEPEST_HEADING),
                    text: plain(section.title()),
                });
                collect(section.body(), level + 1, parts);
            }
            Block::Notice(notice) => parts.push(Part::Notice(notice)),
            Block::Rule(rule) => parts.push(Part::Rule(rule.title().map(plain))),
        }
    }
}

/// The text of `text` with its styling dropped.
pub(super) fn plain(text: &Text) -> String {
    text.spans().iter().map(Span::value).collect()
}

/// `text` without its trailing line breaks, so a block ends in exactly one.
pub(super) fn trimmed(text: &str) -> &str {
    text.trim_end_matches(['\n', '\r'])
}

/// The lowercase name of a notice's level.
pub(super) const fn level_name(level: NoticeLevel) -> &'static str {
    match level {
        NoticeLevel::Success => "success",
        NoticeLevel::Warning => "warning",
        NoticeLevel::Error => "error",
    }
}

/// The cells of a table as the view lays them out. Every row is as wide as
/// the table, which is as wide as its longest row or its header row.
pub(super) struct Grid {
    width: usize,
    header: Option<Vec<String>>,
    rows: Vec<Vec<String>>,
}

impl Grid {
    /// The cells of `table`, with its styling dropped. A table with no
    /// columns has no rows.
    pub(super) fn of(table: &Table) -> Self {
        let width = table
            .rows()
            .iter()
            .map(Vec::len)
            .chain([table.headers().len()])
            .max()
            .unwrap_or_default();
        let cells = |row: &[Text]| {
            let mut cells: Vec<String> = row.iter().map(plain).collect();
            cells.resize(width, String::new());
            cells
        };
        let has_headers = !table.headers().is_empty();
        let rows = if width == 0 {
            Vec::new()
        } else {
            table.rows().iter().map(|row| cells(row)).collect()
        };
        Self {
            width,
            header: has_headers.then(|| cells(table.headers())),
            rows,
        }
    }

    /// The header row, when the table has headers, then each body row.
    pub(super) fn lines(&self) -> impl Iterator<Item = &Vec<String>> {
        self.header.iter().chain(&self.rows)
    }

    /// The header cells, or empty cells for a table with no header row.
    pub(super) fn header_or_blank(&self) -> Vec<String> {
        self.header
            .clone()
            .unwrap_or_else(|| vec![String::new(); self.width])
    }

    /// The body rows.
    pub(super) fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }

    /// Whether the table has no columns, so it writes nothing.
    pub(super) const fn is_empty(&self) -> bool {
        self.width == 0
    }
}

/// The label and value of each row of `fields`.
pub(super) fn records(fields: &Fields) -> Vec<Vec<String>> {
    fields
        .rows()
        .iter()
        .map(|(label, value)| vec![plain(label), plain(value)])
        .collect()
}

/// The blocks that wrote something, as text whose every line ends in a line
/// break, joined by one blank line. `eol` is that line break.
pub(super) fn join(chunks: impl IntoIterator<Item = String>, eol: &str) -> String {
    let mut out = String::new();
    for chunk in chunks.into_iter().filter(|chunk| !chunk.is_empty()) {
        if !out.is_empty() {
            out.push_str(eol);
        }
        out.push_str(&chunk);
    }
    out
}

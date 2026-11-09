//! The semantic document a command's human output is built from.
//!
//! A [`Document`] is an ordered list of [`Block`]s: headings, paragraphs,
//! [`Fields`], [`Table`]s, [`Section`]s, [`Notice`]s, rules, and verbatim
//! text. Text inside a block is a [`Text`] of [`Span`]s, and each span has a
//! [`Role`] that says what it means, not how it looks.
//!
//! The types hold no borders, widths, colours, or streams. The renderer lays
//! a document out and a `Theme` turns each role into a style, so one document
//! renders the same way in every CLI. Enable the `document` feature to build
//! documents without any rendering engine.
//!
//! ```
//! use ctl_core::document::{Document, Fields, Notice, NoticeLevel, Table};
//!
//! let document = Document::new()
//!     .heading("Queue")
//!     .fields(Fields::new().row("pending", "2"))
//!     .table(
//!         Table::new(["id", "title"])
//!             .id_column(0)
//!             .row(["A-1", "Ship"]),
//!     )
//!     .notice(Notice::new(NoticeLevel::Warning, "one row is stale"));
//! assert_eq!(document.blocks().len(), 4);
//! ```

/// An ordered list of blocks that make up a command's human output.
///
/// Every builder method takes `self` and returns it, so a document reads as
/// one chain. A command's `Present` implementation returns one, and a
/// [`Section`] nests one as its body.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Document {
    blocks: Vec<Block>,
}

impl Document {
    /// Start an empty document.
    #[must_use]
    pub const fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    /// Append any value that converts into a [`Block`], such as a [`Fields`],
    /// [`Table`], [`Section`], [`Notice`], or [`Rule`].
    #[must_use]
    pub fn block(mut self, block: impl Into<Block>) -> Self {
        self.blocks.push(block.into());
        self
    }

    /// Append a heading. It is styled with [`Role::Heading`] and wraps to the
    /// width.
    #[must_use]
    pub fn heading(self, value: impl Into<Text>) -> Self {
        self.block(Block::Heading(value.into()))
    }

    /// Append prose that wraps to the width. Spans keep their roles.
    #[must_use]
    pub fn paragraph(self, value: impl Into<Text>) -> Self {
        self.block(Block::Paragraph(value.into()))
    }

    /// Append preformatted text that is never wrapped or styled.
    ///
    /// See [`Block::Verbatim`] for the characters the renderer removes.
    #[must_use]
    pub fn verbatim(self, value: impl Into<String>) -> Self {
        self.block(Block::Verbatim(value.into()))
    }

    /// Append a record of label and value rows.
    #[must_use]
    pub fn fields(self, fields: Fields) -> Self {
        self.block(fields)
    }

    /// Append a table. Its frame follows the table frame of the render options.
    #[must_use]
    pub fn table(self, table: Table) -> Self {
        self.block(table)
    }

    /// Append a section: a heading directly over a nested document.
    #[must_use]
    pub fn section(self, section: Section) -> Self {
        self.block(section)
    }

    /// Append a notice: a level label, an optional code, and a message.
    #[must_use]
    pub fn notice(self, notice: Notice) -> Self {
        self.block(notice)
    }

    /// Append a horizontal rule across the width. A `title` is drawn in the
    /// rule with [`Role::Heading`].
    #[must_use]
    pub fn rule(self, title: Option<Text>) -> Self {
        self.block(Rule { title })
    }

    /// Whether the document has no blocks. A document of blocks that draw
    /// nothing, such as an empty paragraph, is not empty but renders no lines.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// The blocks, in the order they were appended.
    #[must_use]
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }
}

/// One block of a [`Document`].
///
/// Each variant has a builder method on [`Document`] that wraps it. The enum
/// is the read side: a custom renderer matches on it.
///
/// New kinds of block can join the set in a minor release, so a match on it
/// outside ctl-core ends with a wildcard arm.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Block {
    /// A heading, drawn in [`Role::Heading`].
    Heading(Text),
    /// Prose that wraps to the width.
    Paragraph(Text),
    /// Preformatted text whose line breaks survive rendering.
    ///
    /// The renderer removes terminal control characters, bidirectional
    /// controls, and the Unicode line separators U+2028 and U+2029, but keeps
    /// newlines and tabs. It strips newlines from the end, and expands a tab
    /// to the next multiple of 8 cells. Other Unicode formatting characters
    /// stay.
    Verbatim(String),
    /// A record of label and value rows.
    Fields(Fields),
    /// A table, with or without a header row.
    Table(Table),
    /// A heading over a nested document.
    Section(Section),
    /// A level label with an optional code and a message.
    Notice(Notice),
    /// A horizontal rule, optionally titled.
    Rule(Rule),
}

impl From<Fields> for Block {
    fn from(value: Fields) -> Self {
        Self::Fields(value)
    }
}

impl From<Table> for Block {
    fn from(value: Table) -> Self {
        Self::Table(value)
    }
}

impl From<Section> for Block {
    fn from(value: Section) -> Self {
        Self::Section(value)
    }
}

impl From<Notice> for Block {
    fn from(value: Notice) -> Self {
        Self::Notice(value)
    }
}

impl From<Rule> for Block {
    fn from(value: Rule) -> Self {
        Self::Rule(value)
    }
}

/// Text made of [`Span`]s, each carrying a [`Role`].
///
/// A `&str` or `String` converts into plain text, so most methods that take
/// `impl Into<Text>` accept either.
///
/// ```
/// use ctl_core::document::{Role, Text};
///
/// let text = Text::new().token("--force").then(" overwrites ").id("A-1");
/// assert_eq!(text.spans().len(), 3);
/// assert_eq!(text.spans()[0].role(), Role::Token);
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Text {
    spans: Vec<Span>,
}

impl Text {
    /// Start text with no spans.
    #[must_use]
    pub const fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Start text with one [`Role::Plain`] span holding `value`.
    #[must_use]
    pub fn plain(value: impl Into<String>) -> Self {
        Self::new().span(Role::Plain, value)
    }

    /// Append a span holding `value` in `role`.
    #[must_use]
    pub fn span(mut self, role: Role, value: impl Into<String>) -> Self {
        self.spans.push(Span {
            role,
            value: value.into(),
        });
        self
    }

    /// Append `value` in [`Role::Plain`]. A chain of `then` and role methods
    /// reads left to right.
    #[must_use]
    pub fn then(self, value: impl Into<String>) -> Self {
        self.span(Role::Plain, value)
    }

    /// Append `value` in [`Role::Token`]: a flag, command, or field name.
    #[must_use]
    pub fn token(self, value: impl Into<String>) -> Self {
        self.span(Role::Token, value)
    }

    /// Append `value` in [`Role::Id`]: the id of a row, patch, or package.
    #[must_use]
    pub fn id(self, value: impl Into<String>) -> Self {
        self.span(Role::Id, value)
    }

    /// Append `value` in [`Role::Value`]: a value or a metavar.
    #[must_use]
    pub fn value(self, value: impl Into<String>) -> Self {
        self.span(Role::Value, value)
    }

    /// Append `value` in [`Role::Muted`]: secondary information.
    #[must_use]
    pub fn muted(self, value: impl Into<String>) -> Self {
        self.span(Role::Muted, value)
    }

    /// Append `value` in [`Role::Success`].
    #[must_use]
    pub fn success(self, value: impl Into<String>) -> Self {
        self.span(Role::Success, value)
    }

    /// Append `value` in [`Role::Warning`].
    #[must_use]
    pub fn warning(self, value: impl Into<String>) -> Self {
        self.span(Role::Warning, value)
    }

    /// Append `value` in [`Role::Error`].
    #[must_use]
    pub fn error(self, value: impl Into<String>) -> Self {
        self.span(Role::Error, value)
    }

    /// The spans, in the order they were appended.
    #[must_use]
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Whether every span is empty, or there are none. A span of spaces is not
    /// empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(|span| span.value.is_empty())
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self::plain(value)
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self::plain(value)
    }
}

/// A run of text in one [`Role`]. A [`Text`] holds spans, and a span is read
/// through [`Self::role`] and [`Self::value`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Span {
    role: Role,
    value: String,
}

impl Span {
    /// The role that tells the renderer how to style the span.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// The span's text.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// What a [`Span`] means. A `Theme` chooses the style that draws each role.
///
/// The enum is `#[non_exhaustive]`: a `match` needs a wildcard arm.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum Role {
    /// Content with no style of its own. A heading or a token column gives a
    /// plain span its own role.
    #[default]
    Plain,
    /// The text of a heading, a section title, or a table header.
    Heading,
    /// A successful outcome.
    Success,
    /// A warning.
    Warning,
    /// An error.
    Error,
    /// A value or a metavar, such as `<FILE>`.
    Value,
    /// Secondary information that should not draw the eye.
    Muted,
    /// A flag, command, field name, or other literal the user types or reads.
    Token,
    /// The identifier of a row, patch, package, or other domain record.
    Id,
}

/// A record: label and value rows, aligned by the renderer.
///
/// The record style (boxed, keys right, or keys left) comes from the render
/// options.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Fields {
    rows: Vec<(Text, Text)>,
}

impl Fields {
    /// Start a record with no rows.
    #[must_use]
    pub const fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append a row. The `label` is a [`Role::Token`] span; the `value` keeps
    /// the roles it has.
    #[must_use]
    pub fn row(mut self, label: impl Into<String>, value: impl Into<Text>) -> Self {
        self.rows.push((Text::new().token(label), value.into()));
        self
    }

    /// Append a row whose `label` is composed text, for a label that is not a
    /// single token.
    #[must_use]
    pub fn text_row(mut self, label: impl Into<Text>, value: impl Into<Text>) -> Self {
        self.rows.push((label.into(), value.into()));
        self
    }

    /// The label and value of each row, in the order appended.
    #[must_use]
    pub fn rows(&self) -> &[(Text, Text)] {
        &self.rows
    }

    /// Whether the record has no rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Rows of cells under an optional header row.
///
/// The table has as many columns as its longest row or its header row. A
/// shorter row gets empty cells. Column marks, such as [`Self::id_column`],
/// choose the role of a column's plain cells and how it is cut or aligned.
///
/// ```
/// use ctl_core::document::Table;
///
/// let table = Table::new(["id", "title", "age"])
///     .id_column(0)
///     .ellipsis_column(1)
///     .numeric_column(2)
///     .row(["A-1", "Ship the release", "3"]);
/// assert_eq!(table.rows().len(), 1);
/// assert_eq!(table.id_column_index(), Some(0));
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Table {
    headers: Vec<Text>,
    rows: Vec<Vec<Text>>,
    token_column: Option<usize>,
    id_column: Option<usize>,
    ellipsis_columns: Vec<usize>,
    numeric_columns: Vec<usize>,
    stacked_below: Option<Stacked>,
}

impl Table {
    /// Start a table with no header row.
    #[must_use]
    pub fn plain() -> Self {
        Self::default()
    }

    /// Start a table with one header cell per item of `headers`.
    #[must_use]
    pub fn new(headers: impl IntoIterator<Item = impl Into<Text>>) -> Self {
        Self {
            headers: headers.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Draw the plain cells of column `index` (counted from 0) as
    /// [`Role::Token`].
    #[must_use]
    pub const fn token_column(mut self, index: usize) -> Self {
        self.token_column = Some(index);
        self
    }

    /// Draw the plain cells of column `index` (counted from 0) as
    /// [`Role::Id`].
    #[must_use]
    pub const fn id_column(mut self, index: usize) -> Self {
        self.id_column = Some(index);
        self
    }

    /// Mark column `index` (counted from 0) as text that may be cut short.
    ///
    /// A cell too wide for its column keeps one line and ends in `…`. The
    /// column gives up its width before a column of whole words does.
    /// Use it for a title or a summary, never for a value a reader copies
    /// out, such as a URL or a path. Call it once per column.
    #[must_use]
    pub fn ellipsis_column(mut self, index: usize) -> Self {
        self.ellipsis_columns.push(index);
        self
    }

    /// Mark column `index` (counted from 0) as numbers.
    ///
    /// Every cell of the column, header included, aligns right, so numbers
    /// line up on their last digit. Call it once per column.
    #[must_use]
    pub fn numeric_column(mut self, index: usize) -> Self {
        self.numeric_columns.push(index);
        self
    }

    /// Stack each row when the available width is below `width` cells.
    ///
    /// The first `label_columns` cells of a row form its label line. The
    /// remaining cells form the indented description under it. The label cells
    /// are drawn as [`Role::Token`].
    #[must_use]
    pub const fn stacked_below(mut self, width: u16, label_columns: usize) -> Self {
        self.stacked_below = Some(Stacked {
            width,
            label_columns,
        });
        self
    }

    /// Append a row of cells, one per item.
    #[must_use]
    pub fn row(mut self, cells: impl IntoIterator<Item = impl Into<Text>>) -> Self {
        self.rows.push(cells.into_iter().map(Into::into).collect());
        self
    }

    /// The header cells, empty for a table with no header row.
    #[must_use]
    pub fn headers(&self) -> &[Text] {
        &self.headers
    }

    /// The body rows, in the order appended.
    #[must_use]
    pub fn rows(&self) -> &[Vec<Text>] {
        &self.rows
    }

    /// The column set by [`Self::token_column`], if any.
    #[must_use]
    pub const fn token_column_index(&self) -> Option<usize> {
        self.token_column
    }

    /// The column set by [`Self::id_column`], if any.
    #[must_use]
    pub const fn id_column_index(&self) -> Option<usize> {
        self.id_column
    }

    /// Columns marked with [`Self::ellipsis_column`], in the order marked.
    #[must_use]
    pub fn ellipsis_columns(&self) -> &[usize] {
        &self.ellipsis_columns
    }

    /// Columns marked with [`Self::numeric_column`], in the order marked.
    #[must_use]
    pub fn numeric_columns(&self) -> &[usize] {
        &self.numeric_columns
    }

    /// The narrow-layout policy set by [`Self::stacked_below`], if any.
    #[must_use]
    pub const fn stacked(&self) -> Option<Stacked> {
        self.stacked_below
    }

    /// Whether the table has no body rows. A header row does not count.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// When a [`Table`] stacks its rows, and how many cells form each label.
///
/// Read it from [`Table::stacked`]; [`Table::stacked_below`] sets it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stacked {
    width: u16,
    label_columns: usize,
}

impl Stacked {
    /// The width, in cells, below which rows stack.
    #[must_use]
    pub const fn width(self) -> u16 {
        self.width
    }

    /// The number of leading cells that form a stacked row's label.
    #[must_use]
    pub const fn label_columns(self) -> usize {
        self.label_columns
    }
}

/// A heading over a nested [`Document`].
///
/// The heading sits directly over the body with no blank line and no indent.
/// A table in the body then lines up with the tables of the outer document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Section {
    title: Text,
    body: Document,
}

impl Section {
    /// Build a section from its `title` and `body`.
    #[must_use]
    pub fn new(title: impl Into<Text>, body: Document) -> Self {
        Self {
            title: title.into(),
            body,
        }
    }

    /// The heading text.
    #[must_use]
    pub const fn title(&self) -> &Text {
        &self.title
    }

    /// The nested document under the heading.
    #[must_use]
    pub const fn body(&self) -> &Document {
        &self.body
    }
}

/// A one-line message with a level label, such as `warning`, and an optional
/// code.
///
/// The renderer draws the label, then the code if there is one, then the
/// message, separated by ` · `.
///
/// ```
/// use ctl_core::document::{Notice, NoticeLevel};
///
/// let notice = Notice::new(NoticeLevel::Error, "no such row").code("not-found");
/// assert_eq!(notice.code_value(), Some("not-found"));
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notice {
    level: NoticeLevel,
    code: Option<String>,
    message: Text,
}

impl Notice {
    /// Build a notice of `level` with no code.
    #[must_use]
    pub fn new(level: NoticeLevel, message: impl Into<Text>) -> Self {
        Self {
            level,
            code: None,
            message: message.into(),
        }
    }

    /// Attach a short code that stays the same for the same kind of notice, so
    /// a reader can search for it.
    #[must_use]
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// The severity, which picks the label and its style.
    #[must_use]
    pub const fn level(&self) -> NoticeLevel {
        self.level
    }

    /// The code set by [`Self::code`], if any.
    #[must_use]
    pub fn code_value(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// The message text.
    #[must_use]
    pub const fn message(&self) -> &Text {
        &self.message
    }
}

/// The severity of a [`Notice`]. It picks the label (`success`, `warning`, or
/// `error`) and the style of the label.
///
/// A level can join the set in a minor release, so a match on it outside
/// ctl-core ends with a wildcard arm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum NoticeLevel {
    /// A successful outcome.
    Success,
    /// A warning that does not fail the command.
    Warning,
    /// A failed outcome.
    Error,
}

/// A horizontal rule across the width, optionally titled.
///
/// Build one with [`Document::rule`]. This type is the read side.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    title: Option<Text>,
}

impl Rule {
    /// The title drawn in the rule, if any.
    #[must_use]
    pub const fn title(&self) -> Option<&Text> {
        self.title.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::{Document, Fields, Notice, NoticeLevel, Role, Table, Text};

    #[test]
    fn fluent_document_preserves_semantics() {
        let document = Document::new()
            .heading("status")
            .fields(Fields::new().row("pending", "2"))
            .table(
                Table::new(["id", "title"])
                    .token_column(0)
                    .row(["A-1", "Ship"]),
            )
            .notice(Notice::new(NoticeLevel::Warning, "stale"));

        assert_eq!(document.blocks().len(), 4);
        let token = Text::new().token("--force").then(" writes");
        assert_eq!(token.spans()[0].role(), Role::Token);
        assert_eq!(token.spans()[1].role(), Role::Plain);
    }

    #[test]
    fn column_marks_accumulate() {
        let table = Table::new(["id", "title", "age", "size"])
            .ellipsis_column(1)
            .numeric_column(2)
            .numeric_column(3);
        assert_eq!(table.ellipsis_columns(), [1]);
        assert_eq!(table.numeric_columns(), [2, 3]);
    }
}

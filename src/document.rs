//! Semantic presentation document shared by every ctl CLI.
//!
//! Consumers describe information with these types. Terminal layout, borders,
//! wrapping, color, and streams remain ctl-core implementation details.

/// A composable human presentation.
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

    /// Append one semantic block.
    #[must_use]
    pub fn block(mut self, block: impl Into<Block>) -> Self {
        self.blocks.push(block.into());
        self
    }

    /// Append a heading.
    #[must_use]
    pub fn heading(self, value: impl Into<Text>) -> Self {
        self.block(Block::Heading(value.into()))
    }

    /// Append wrapped prose.
    #[must_use]
    pub fn paragraph(self, value: impl Into<Text>) -> Self {
        self.block(Block::Paragraph(value.into()))
    }

    /// Append preformatted text without wrapping or semantic styling.
    #[must_use]
    pub fn verbatim(self, value: impl Into<String>) -> Self {
        self.block(Block::Verbatim(value.into()))
    }

    /// Append key/value fields.
    #[must_use]
    pub fn fields(self, fields: Fields) -> Self {
        self.block(fields)
    }

    /// Append a grid.
    #[must_use]
    pub fn table(self, table: Table) -> Self {
        self.block(table)
    }

    /// Append a titled section.
    #[must_use]
    pub fn section(self, section: Section) -> Self {
        self.block(section)
    }

    /// Append a semantic notice.
    #[must_use]
    pub fn notice(self, notice: Notice) -> Self {
        self.block(notice)
    }

    /// Append a horizontal rule, optionally titled.
    #[must_use]
    pub fn rule(self, title: Option<Text>) -> Self {
        self.block(Rule { title })
    }

    /// Whether this document has no blocks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Semantic blocks in document order.
    #[must_use]
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }
}

/// One semantic presentation block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Block {
    /// Section-level heading.
    Heading(Text),
    /// Wrapped prose.
    Paragraph(Text),
    /// Preformatted text kept unwrapped inside the document newline contract.
    /// Unsafe control characters are removed; tabs and line breaks survive.
    Verbatim(String),
    /// Key/value fields.
    Fields(Fields),
    /// Headered or headerless table.
    Table(Table),
    /// Heading plus nested document.
    Section(Section),
    /// Success, warning, or error notice.
    Notice(Notice),
    /// Horizontal divider.
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

/// Styled text assembled from semantic spans.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Text {
    spans: Vec<Span>,
}

impl Text {
    /// Start empty text.
    #[must_use]
    pub const fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Start plain text.
    #[must_use]
    pub fn plain(value: impl Into<String>) -> Self {
        Self::new().span(Role::Plain, value)
    }

    /// Append one semantic span.
    #[must_use]
    pub fn span(mut self, role: Role, value: impl Into<String>) -> Self {
        self.spans.push(Span {
            role,
            value: value.into(),
        });
        self
    }

    /// Append plain text.
    #[must_use]
    pub fn then(self, value: impl Into<String>) -> Self {
        self.span(Role::Plain, value)
    }

    /// Append a semantic token such as a flag, command, or field name.
    #[must_use]
    pub fn token(self, value: impl Into<String>) -> Self {
        self.span(Role::Token, value)
    }

    /// Append a value or metavar.
    #[must_use]
    pub fn value(self, value: impl Into<String>) -> Self {
        self.span(Role::Value, value)
    }

    /// Append secondary text.
    #[must_use]
    pub fn muted(self, value: impl Into<String>) -> Self {
        self.span(Role::Muted, value)
    }

    /// Append success text.
    #[must_use]
    pub fn success(self, value: impl Into<String>) -> Self {
        self.span(Role::Success, value)
    }

    /// Append warning text.
    #[must_use]
    pub fn warning(self, value: impl Into<String>) -> Self {
        self.span(Role::Warning, value)
    }

    /// Append error text.
    #[must_use]
    pub fn error(self, value: impl Into<String>) -> Self {
        self.span(Role::Error, value)
    }

    /// Semantic spans in order.
    #[must_use]
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Whether this text has no spans or visible characters.
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

/// One semantic text span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Span {
    role: Role,
    value: String,
}

impl Span {
    /// Semantic role.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// Text content.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Meaning carried by a text span. A renderer chooses the visual style.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Role {
    /// Unstyled content.
    #[default]
    Plain,
    /// Heading.
    Heading,
    /// Successful outcome.
    Success,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// Value or metavar.
    Value,
    /// Secondary information.
    Muted,
    /// Flag, command, field name, or other operator token.
    Token,
}

/// Key/value rows.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Fields {
    rows: Vec<(Text, Text)>,
}

impl Fields {
    /// Start empty fields.
    #[must_use]
    pub const fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append one field. Labels receive token semantics automatically.
    #[must_use]
    pub fn row(mut self, label: impl Into<String>, value: impl Into<Text>) -> Self {
        self.rows.push((Text::new().token(label), value.into()));
        self
    }

    /// Append one field with an explicitly composed label.
    #[must_use]
    pub fn text_row(mut self, label: impl Into<Text>, value: impl Into<Text>) -> Self {
        self.rows.push((label.into(), value.into()));
        self
    }

    /// Rows in display order.
    #[must_use]
    pub fn rows(&self) -> &[(Text, Text)] {
        &self.rows
    }

    /// Whether no rows exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// A semantic table independent of its rendering engine.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Table {
    headers: Vec<Text>,
    rows: Vec<Vec<Text>>,
    token_column: Option<usize>,
    stacked_below: Option<Stacked>,
}

impl Table {
    /// Start a headerless table.
    #[must_use]
    pub fn plain() -> Self {
        Self::default()
    }

    /// Start a table with headers.
    #[must_use]
    pub fn new(headers: impl IntoIterator<Item = impl Into<Text>>) -> Self {
        Self {
            headers: headers.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Style one column as operator tokens.
    #[must_use]
    pub const fn token_column(mut self, index: usize) -> Self {
        self.token_column = Some(index);
        self
    }

    /// Stack labels and descriptions when the available width is below `width`.
    ///
    /// `label_columns` controls how many leading cells form the label. The
    /// remaining cells form the indented description.
    #[must_use]
    pub const fn stacked_below(mut self, width: u16, label_columns: usize) -> Self {
        self.stacked_below = Some(Stacked {
            width,
            label_columns,
        });
        self
    }

    /// Append one row.
    #[must_use]
    pub fn row(mut self, cells: impl IntoIterator<Item = impl Into<Text>>) -> Self {
        self.rows.push(cells.into_iter().map(Into::into).collect());
        self
    }

    /// Headers in order.
    #[must_use]
    pub fn headers(&self) -> &[Text] {
        &self.headers
    }

    /// Rows in order.
    #[must_use]
    pub fn rows(&self) -> &[Vec<Text>] {
        &self.rows
    }

    /// Column that carries token semantics.
    #[must_use]
    pub const fn token_column_index(&self) -> Option<usize> {
        self.token_column
    }

    /// Narrow-layout policy.
    #[must_use]
    pub const fn stacked(&self) -> Option<Stacked> {
        self.stacked_below
    }

    /// Whether no rows exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Narrow table layout policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stacked {
    width: u16,
    label_columns: usize,
}

impl Stacked {
    /// Width below which rows stack.
    #[must_use]
    pub const fn width(self) -> u16 {
        self.width
    }

    /// Number of leading label cells.
    #[must_use]
    pub const fn label_columns(self) -> usize {
        self.label_columns
    }
}

/// A titled nested document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Section {
    title: Text,
    body: Document,
}

impl Section {
    /// Build a section.
    #[must_use]
    pub fn new(title: impl Into<Text>, body: Document) -> Self {
        Self {
            title: title.into(),
            body,
        }
    }

    /// Section title.
    #[must_use]
    pub const fn title(&self) -> &Text {
        &self.title
    }

    /// Section body.
    #[must_use]
    pub const fn body(&self) -> &Document {
        &self.body
    }
}

/// A semantic notice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notice {
    level: NoticeLevel,
    code: Option<String>,
    message: Text,
}

impl Notice {
    /// Build a notice.
    #[must_use]
    pub fn new(level: NoticeLevel, message: impl Into<Text>) -> Self {
        Self {
            level,
            code: None,
            message: message.into(),
        }
    }

    /// Attach a stable notice code.
    #[must_use]
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Notice severity.
    #[must_use]
    pub const fn level(&self) -> NoticeLevel {
        self.level
    }

    /// Optional stable code.
    #[must_use]
    pub fn code_value(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Human message.
    #[must_use]
    pub const fn message(&self) -> &Text {
        &self.message
    }
}

/// Notice severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoticeLevel {
    /// Successful outcome.
    Success,
    /// Warning that does not fail the command.
    Warning,
    /// Failed outcome.
    Error,
}

/// A horizontal divider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    title: Option<Text>,
}

impl Rule {
    /// Optional title.
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
}

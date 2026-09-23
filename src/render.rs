//! Render semantic documents without exposing the terminal engine.

use std::fmt::Write as _;

use comfy_table::presets::{NOTHING, UTF8_FULL, UTF8_FULL_CONDENSED};
use comfy_table::{
    Cell, CellAlignment, ContentArrangement, LineStyle, Table as EngineTable, TableStyle,
};
use unicode_bidi::format_chars::{ALM, FSI, LRE, LRI, LRM, LRO, PDF, PDI, RLE, RLI, RLM, RLO};
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_width::UnicodeWidthStr;

use crate::color::ColorMode;
use crate::document::{Block, Document, Fields, Notice, NoticeLevel, Role, Section, Table, Text};
use crate::style::{ERROR, HEADING, ID, MUTED, OPTION, SUCCESS, VALUE, WARNING, styled};

/// Default columns reserved from an automatically detected terminal width.
pub const DEFAULT_COLUMN_BUFFER: u16 = 2;
/// Default environment variable for overriding the automatic-width buffer.
pub const DEFAULT_COLUMN_BUFFER_ENV: &str = "CTL_CORE_COLUMN_BUFFER";
/// Default ordered environment lookup for the automatic-width buffer.
pub const DEFAULT_COLUMN_BUFFER_ENVS: &[&str] = &[DEFAULT_COLUMN_BUFFER_ENV];
/// Default floor for an automatically detected effective width.
pub const DEFAULT_MINIMUM_AUTOMATIC_WIDTH: u16 = 20;
/// Width used when neither the terminal nor `COLUMNS` gives one.
pub const DEFAULT_FALLBACK_WIDTH: u16 = 80;

/// A light horizontal rule that runs through column gaps.
const RULE: LineStyle = LineStyle::none().fill('─').junction('─');

/// How a [`Fields`] record is framed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RecordStyle {
    /// A full box around every key and value.
    Boxed,
    /// No frame. Keys are right-aligned in one column.
    #[default]
    KeysRight,
    /// No frame. Keys are left-aligned in one column.
    KeysLeft,
}

/// How a [`Table`] of rows is framed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ListStyle {
    /// Outer frame, column rules, and a header rule.
    #[default]
    Grid,
    /// One rule under the header and nothing else.
    HeaderRule,
    /// No rules at all.
    Plain,
}

/// What separates consecutive rows of a [`Table`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RowSeparation {
    /// Rows print on consecutive lines.
    #[default]
    None,
    /// A horizontal rule between each pair of rows.
    Rule,
    /// A blank line between each pair of rows.
    Blank,
}

/// Deterministic document rendering options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    color: ColorMode,
    width: Option<u16>,
    automatic_width_buffer: Option<u16>,
    automatic_width_buffer_envs: &'static [&'static str],
    minimum_automatic_width: u16,
    fallback_width: Option<u16>,
    record_style: RecordStyle,
    list_style: ListStyle,
    row_separation: RowSeparation,
}

impl RenderOptions {
    /// Build options with automatic terminal width, a
    /// [`DEFAULT_COLUMN_BUFFER`]-column buffer, [`DEFAULT_COLUMN_BUFFER_ENV`]
    /// as the operator override, and [`DEFAULT_FALLBACK_WIDTH`] when no
    /// width is detected.
    #[must_use]
    pub const fn new(color: ColorMode) -> Self {
        Self {
            color,
            width: None,
            automatic_width_buffer: None,
            automatic_width_buffer_envs: DEFAULT_COLUMN_BUFFER_ENVS,
            minimum_automatic_width: DEFAULT_MINIMUM_AUTOMATIC_WIDTH,
            fallback_width: Some(DEFAULT_FALLBACK_WIDTH),
            record_style: RecordStyle::KeysRight,
            list_style: ListStyle::Grid,
            row_separation: RowSeparation::None,
        }
    }

    /// Frame [`Fields`] records in `style`.
    #[must_use]
    pub const fn record_style(mut self, style: RecordStyle) -> Self {
        self.record_style = style;
        self
    }

    /// Frame [`Table`] lists in `style`.
    #[must_use]
    pub const fn list_style(mut self, style: ListStyle) -> Self {
        self.list_style = style;
        self
    }

    /// Separate [`Table`] rows with `separation`.
    #[must_use]
    pub const fn row_separation(mut self, separation: RowSeparation) -> Self {
        self.row_separation = separation;
        self
    }

    /// Record framing.
    #[must_use]
    pub const fn record(self) -> RecordStyle {
        self.record_style
    }

    /// List framing.
    #[must_use]
    pub const fn list(self) -> ListStyle {
        self.list_style
    }

    /// Row separation.
    #[must_use]
    pub const fn separation(self) -> RowSeparation {
        self.row_separation
    }

    /// Force an explicit width. Tests and redirected renderers should use this.
    #[must_use]
    pub const fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Override the automatic-width buffer. Zero explicitly disables it.
    /// Explicit [`Self::width`] remains exact regardless of this value.
    #[must_use]
    pub const fn automatic_width_buffer(mut self, columns: u16) -> Self {
        self.automatic_width_buffer = Some(columns);
        self
    }

    /// Replace the ordered environment names used to configure the buffer.
    /// Pass aliases in precedence order or an empty slice to disable lookup.
    #[must_use]
    pub const fn automatic_width_buffer_envs(mut self, names: &'static [&'static str]) -> Self {
        self.automatic_width_buffer_envs = names;
        self
    }

    /// Set the floor for automatically detected effective widths.
    #[must_use]
    pub const fn minimum_automatic_width(mut self, columns: u16) -> Self {
        self.minimum_automatic_width = columns;
        self
    }

    /// Lay out to `width` when neither the terminal nor `COLUMNS` gives one.
    /// `None` renders tables at their natural width instead. This does not
    /// touch [`Self::minimum_automatic_width`], which applies only to a
    /// detected width.
    #[must_use]
    pub const fn fallback_width(mut self, width: Option<u16>) -> Self {
        self.fallback_width = width;
        self
    }

    /// Width used when none is detected, if any.
    #[must_use]
    pub const fn fallback(self) -> Option<u16> {
        self.fallback_width
    }

    /// Color policy.
    #[must_use]
    pub const fn color(self) -> ColorMode {
        self.color
    }

    /// Explicit width, when set.
    #[must_use]
    pub const fn explicit_width(self) -> Option<u16> {
        self.width
    }

    /// Explicit automatic-width buffer, when the library owner set one.
    #[must_use]
    pub const fn explicit_automatic_width_buffer(self) -> Option<u16> {
        self.automatic_width_buffer
    }

    /// Ordered environment names used for the automatic-width buffer.
    #[must_use]
    pub const fn automatic_width_buffer_env_names(self) -> &'static [&'static str] {
        self.automatic_width_buffer_envs
    }

    /// Floor applied only to automatically detected widths.
    #[must_use]
    pub const fn automatic_width_minimum(self) -> u16 {
        self.minimum_automatic_width
    }

    fn resolved_automatic_width_buffer(self) -> u16 {
        self.automatic_width_buffer
            .or_else(|| crate::layout::column_buffer(self.automatic_width_buffer_envs))
            .unwrap_or(DEFAULT_COLUMN_BUFFER)
    }
}

/// Semantic document renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Renderer {
    options: RenderOptions,
}

impl Renderer {
    /// Build a renderer.
    #[must_use]
    pub const fn new(options: RenderOptions) -> Self {
        Self { options }
    }

    /// Render one document to a newline-terminated string.
    #[must_use]
    pub fn render(self, document: &Document) -> String {
        let mut rendered = document
            .blocks()
            .iter()
            .filter_map(|block| {
                let rendered = self.render_block(block);
                (!rendered.is_empty()).then_some(rendered)
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        if !rendered.is_empty() {
            rendered.push('\n');
        }
        rendered
    }

    fn render_block(self, block: &Block) -> String {
        match block {
            Block::Heading(text) => self.wrap(&self.text_with_default(text, Role::Heading), 0),
            Block::Paragraph(text) => self.wrap(&self.text(text), 0),
            Block::Verbatim(value) => sanitize_verbatim(value),
            Block::Fields(fields) => self.fields(fields),
            Block::Table(table) => self.table(table),
            Block::Section(section) => self.section(section),
            Block::Notice(notice) => self.notice(notice),
            Block::Rule(rule) => {
                let width = usize::from(self.width().unwrap_or(40));
                rule.title().map_or_else(
                    || "─".repeat(width),
                    |title| {
                        let title_width = title
                            .spans()
                            .iter()
                            .map(|span| UnicodeWidthStr::width(span.value()))
                            .sum::<usize>();
                        let title = self.text_with_default(title, Role::Heading);
                        format!(
                            "── {title} {}",
                            "─".repeat(width.saturating_sub(title_width + 4))
                        )
                    },
                )
            }
        }
    }

    fn section(self, section: &Section) -> String {
        let heading = self.wrap(&self.text_with_default(section.title(), Role::Heading), 0);
        let body = self.render(section.body());
        if body.is_empty() {
            heading
        } else {
            format!("{heading}\n{}", body.trim_end())
        }
    }

    fn fields(self, fields: &Fields) -> String {
        let style = self.options.record();
        let mut table = self.engine_table(if style == RecordStyle::Boxed {
            UTF8_FULL_CONDENSED
        } else {
            NOTHING
        });
        for (label, value) in fields.rows() {
            table.add_row([self.text(label), self.text(value)]);
        }
        if style != RecordStyle::Boxed {
            if let Some(keys) = table.column_mut(0) {
                keys.set_padding((0, 1));
                if style == RecordStyle::KeysRight {
                    keys.set_cell_alignment(CellAlignment::Right);
                }
            }
            if let Some(values) = table.column_mut(1) {
                values.set_padding((1, 0));
            }
        }
        trim_line_ends(&table.to_string())
    }

    fn table(self, table: &Table) -> String {
        if self.should_stack(table) {
            return self.stacked(table);
        }
        let mut engine = self.list_table();
        if !table.headers().is_empty() {
            engine.set_header(
                table
                    .headers()
                    .iter()
                    .map(|header| Cell::new(self.text_with_default(header, Role::Heading))),
            );
        }
        let blank = self.options.separation() == RowSeparation::Blank;
        for (position, row) in table.rows().iter().enumerate() {
            if blank && position > 0 {
                engine.add_row(row.iter().map(|_| Cell::new("")));
            }
            engine.add_row(row.iter().enumerate().map(|(index, cell)| {
                let value = if table.token_column_index() == Some(index) {
                    self.text_with_default(cell, Role::Token)
                } else if table.id_column_index() == Some(index) {
                    self.text_with_default(cell, Role::Id)
                } else {
                    self.text(cell)
                };
                Cell::new(value)
            }));
        }
        if self.options.list() != ListStyle::Grid
            && let Some(first) = engine.column_mut(0)
        {
            first.set_padding((0, 1));
        }
        trim_line_ends(&engine.to_string())
    }

    fn list_table(self) -> EngineTable {
        let rule = self.options.separation() == RowSeparation::Rule;
        let style = match (self.options.list(), rule) {
            (ListStyle::Grid, false) => UTF8_FULL_CONDENSED,
            (ListStyle::Grid, true) => UTF8_FULL,
            (ListStyle::HeaderRule, false) => NOTHING.header_separator(RULE),
            (ListStyle::HeaderRule, true) => NOTHING.header_separator(RULE).row_separator(RULE),
            (ListStyle::Plain, false) => NOTHING,
            (ListStyle::Plain, true) => NOTHING.row_separator(RULE),
        };
        self.engine_table(style)
    }

    fn should_stack(self, table: &Table) -> bool {
        let Some(stacked) = table.stacked() else {
            return false;
        };
        self.width().is_some_and(|width| width < stacked.width())
    }

    fn stacked(self, table: &Table) -> String {
        let Some(policy) = table.stacked() else {
            return String::new();
        };
        let mut output = String::new();
        for row in table.rows() {
            let labels = row
                .iter()
                .take(policy.label_columns())
                .filter(|value| !value.is_empty())
                .map(|value| self.text_with_default(value, Role::Token))
                .collect::<Vec<_>>()
                .join(" ");
            let description = row
                .iter()
                .skip(policy.label_columns())
                .filter(|value| !value.is_empty())
                .map(|value| self.text(value))
                .collect::<Vec<_>>()
                .join(" ");
            for line in self.wrap(&labels, 2).lines() {
                let _ = writeln!(output, "  {line}");
            }
            if !description.is_empty() {
                let wrapped = self.wrap(&description, 4);
                for line in wrapped.lines() {
                    let _ = writeln!(output, "    {line}");
                }
            }
        }
        output.trim_end().to_owned()
    }

    fn notice(self, notice: &Notice) -> String {
        let (label, role) = match notice.level() {
            NoticeLevel::Success => ("success", Role::Success),
            NoticeLevel::Warning => ("warning", Role::Warning),
            NoticeLevel::Error => ("error", Role::Error),
        };
        let mut line = self.paint(role, label);
        if let Some(code) = notice.code_value() {
            let _ = write!(line, " · {}", self.paint(Role::Muted, code));
        }
        let _ = write!(line, " · {}", self.text(notice.message()));
        self.wrap(&line, 0)
    }

    fn engine_table(self, preset: TableStyle) -> EngineTable {
        let mut table = EngineTable::new();
        table
            .load_style(preset)
            .set_content_arrangement(ContentArrangement::Dynamic);
        if let Some(width) = self.width() {
            table.set_width(width);
        }
        table
    }

    fn wrap(self, value: &str, indentation: u16) -> String {
        let Some(width) = self.width() else {
            return value.to_owned();
        };
        let mut table = EngineTable::new();
        table
            .load_style(NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(width.saturating_sub(indentation))
            .add_row([value]);
        if let Some(column) = table.column_mut(0) {
            column.set_padding((0, 0));
        }
        table
            .to_string()
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn width(self) -> Option<u16> {
        self.options.explicit_width().or_else(|| {
            crate::layout::terminal_width(
                self.options.resolved_automatic_width_buffer(),
                self.options.automatic_width_minimum(),
            )
            .or(self.options.fallback())
        })
    }

    fn text(self, text: &Text) -> String {
        text.spans()
            .iter()
            .map(|span| self.paint(span.role(), span.value()))
            .collect()
    }

    fn text_with_default(self, text: &Text, default: Role) -> String {
        text.spans()
            .iter()
            .map(|span| {
                let role = if span.role() == Role::Plain {
                    default
                } else {
                    span.role()
                };
                self.paint(role, span.value())
            })
            .collect()
    }

    fn paint(self, role: Role, value: &str) -> String {
        if self.options.color() == ColorMode::Never || role == Role::Plain {
            return value.to_owned();
        }
        let style = match role {
            Role::Plain => return value.to_owned(),
            Role::Heading => HEADING,
            Role::Success => SUCCESS,
            Role::Warning => WARNING,
            Role::Error => ERROR,
            Role::Value => VALUE,
            Role::Muted => MUTED,
            Role::Token => OPTION,
            Role::Id => ID,
        };
        styled(style, value)
    }
}

impl Document {
    /// Render with explicit semantic options.
    #[must_use]
    pub fn render(&self, options: RenderOptions) -> String {
        Renderer::new(options).render(self)
    }
}

/// Borderless styles leave cell padding at the line end; drop it so captured
/// output has no trailing spaces.
fn trim_line_ends(value: &str) -> String {
    value
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_verbatim(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            *character == '\n' || *character == '\t' || !is_unsafe_verbatim(*character)
        })
        .collect::<String>()
        .trim_end_matches('\n')
        .to_owned()
}

fn is_unsafe_verbatim(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            ALM | FSI | LRE | LRI | LRM | LRO | PDF | PDI | RLE | RLI | RLM | RLO
        )
        || matches!(
            get_general_category(character),
            GeneralCategory::LineSeparator | GeneralCategory::ParagraphSeparator
        )
}

#[cfg(test)]
mod tests {
    use indoc::{formatdoc, indoc};

    use super::{ListStyle, RecordStyle, RenderOptions, RowSeparation};
    use crate::color::ColorMode;
    use crate::document::{Document, Fields, Notice, NoticeLevel, Section, Table, Text};

    #[test]
    fn automatic_width_policy_is_explicit_and_overridable() {
        let defaults = RenderOptions::new(ColorMode::Never);
        assert_eq!(defaults.explicit_automatic_width_buffer(), None);
        assert_eq!(
            defaults.automatic_width_minimum(),
            super::DEFAULT_MINIMUM_AUTOMATIC_WIDTH
        );

        let configured = defaults
            .automatic_width_buffer(0)
            .automatic_width_buffer_envs(&["APP_COLUMNS_BUFFER", "LEGACY_BUFFER"])
            .minimum_automatic_width(8);
        assert_eq!(configured.explicit_automatic_width_buffer(), Some(0));
        assert_eq!(
            configured.automatic_width_buffer_env_names(),
            ["APP_COLUMNS_BUFFER", "LEGACY_BUFFER"]
        );
        assert_eq!(configured.automatic_width_minimum(), 8);
    }

    #[test]
    fn explicit_width_ignores_the_automatic_buffer() {
        let document = Document::new().paragraph("one two three four five six");
        let exact = document.render(RenderOptions::new(ColorMode::Never).width(16));
        let buffered = document.render(
            RenderOptions::new(ColorMode::Never)
                .width(16)
                .automatic_width_buffer(8),
        );
        assert_eq!(buffered, exact);
    }

    #[test]
    fn colorless_document_is_deterministic() {
        let document = Document::new()
            .heading("status")
            .fields(Fields::new().row("pending", Text::plain("2")))
            .notice(Notice::new(NoticeLevel::Warning, "one stale row"));
        let rendered = document.render(RenderOptions::new(ColorMode::Never).width(60));
        let expected = indoc! {"
            status

            pending  2

            warning · one stale row
        "};
        assert_eq!(rendered, expected);
        assert!(!rendered.contains('\u{1b}'));
    }

    fn queue() -> Table {
        Table::new(["id", "title"])
            .row(["QCTL-014", "Let a ledger declare row separation"])
            .row(["QCTL-015", "Write a ledger atomically"])
            .row(["QCTL-016", "Name the next startable row"])
    }

    #[test]
    fn borderless_record_right_aligns_keys() {
        let rendered = Document::new()
            .fields(
                Fields::new()
                    .row("ledger", Text::plain("tasks.yaml"))
                    .row("active", Text::plain("QCTL-014")),
            )
            .render(
                RenderOptions::new(ColorMode::Never)
                    .width(80)
                    .record_style(RecordStyle::KeysRight),
            );
        let expected = indoc! {"
            ledger  tasks.yaml
            active  QCTL-014
        "};
        assert_eq!(rendered, expected);
    }

    #[test]
    fn left_aligned_record_pads_short_keys_on_the_right() {
        let rendered = Document::new()
            .fields(
                Fields::new()
                    .row("id", Text::plain("QCTL-014"))
                    .row("outcome", Text::plain("done")),
            )
            .render(
                RenderOptions::new(ColorMode::Never)
                    .width(80)
                    .record_style(RecordStyle::KeysLeft),
            );
        let expected = indoc! {"
            id       QCTL-014
            outcome  done
        "};
        assert_eq!(rendered, expected);
    }

    #[test]
    fn header_rule_draws_one_rule_and_no_verticals() {
        let rendered = Document::new().table(queue()).render(
            RenderOptions::new(ColorMode::Never)
                .width(80)
                .list_style(ListStyle::HeaderRule),
        );
        let rules = rendered
            .lines()
            .filter(|line| line.starts_with('─'))
            .count();
        assert_eq!(rules, 1, "{rendered}");
        assert_eq!(
            rendered.lines().nth(1).map(|line| line.starts_with('─')),
            Some(true)
        );
        assert!(!rendered.contains(['│', '┆', '┌', '└']), "{rendered}");
    }

    #[test]
    fn rule_separation_draws_a_rule_between_rows() {
        let rendered = Document::new().table(queue()).render(
            RenderOptions::new(ColorMode::Never)
                .width(80)
                .list_style(ListStyle::Plain)
                .row_separation(RowSeparation::Rule),
        );
        let lines = rendered.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 6, "{rendered}");
        assert!(
            lines[2].starts_with('─') && lines[4].starts_with('─'),
            "{rendered}"
        );
    }

    #[test]
    fn blank_separation_leaves_one_empty_line_between_rows() {
        let rendered = Document::new().table(queue()).render(
            RenderOptions::new(ColorMode::Never)
                .width(80)
                .list_style(ListStyle::Plain)
                .row_separation(RowSeparation::Blank),
        );
        let lines = rendered.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 6, "{rendered}");
        assert!(lines[2].is_empty() && lines[4].is_empty(), "{rendered}");
    }

    #[test]
    fn defaults_are_the_operator_picks() {
        let options = RenderOptions::new(ColorMode::Never);
        assert_eq!(options.record(), RecordStyle::KeysRight);
        assert_eq!(options.list(), ListStyle::Grid);
        assert_eq!(options.separation(), RowSeparation::None);
        assert_eq!(options.fallback(), Some(80));
        assert_eq!(options.fallback_width(None).fallback(), None);
        assert_eq!(super::DEFAULT_COLUMN_BUFFER, 2);
    }

    #[test]
    fn colored_document_has_ansi() {
        let document = Document::new().heading("status");
        let rendered = document.render(RenderOptions::new(ColorMode::Always).width(60));
        assert!(rendered.contains('\u{1b}'));
    }

    #[test]
    fn ids_are_bold_without_a_colour() {
        let table = Table::new(["id", "title"])
            .id_column(0)
            .row(["QCTL-014", "Title"]);
        let rendered = Document::new()
            .table(table)
            .paragraph(Text::new().id("QCTL-015"))
            .render(RenderOptions::new(ColorMode::Always).width(60));
        assert!(
            rendered.contains("\u{1b}[1mQCTL-014\u{1b}[0m"),
            "{rendered}"
        );
        assert!(
            rendered.contains("\u{1b}[1mQCTL-015\u{1b}[0m"),
            "{rendered}"
        );
    }

    #[test]
    fn headings_wrap_to_the_width() {
        let rendered = Document::new()
            .heading("QCTL-001  a title that runs past the line")
            .section(Section::new(
                "a section title that also runs long",
                Document::new(),
            ))
            .render(RenderOptions::new(ColorMode::Never).width(20));
        assert!(rendered.lines().count() > 2, "{rendered}");
        assert!(
            rendered.lines().all(|line| line.chars().count() <= 20),
            "{rendered}"
        );
    }

    #[test]
    fn narrow_table_stacks() {
        let table =
            Table::plain()
                .stacked_below(64, 2)
                .row(["-f", "--format", "Output representation"]);
        let rendered = Document::new()
            .table(table)
            .render(RenderOptions::new(ColorMode::Never).width(40));
        assert_eq!(rendered, "  -f --format\n    Output representation\n");
    }

    #[test]
    fn verbatim_text_ignores_explicit_width() {
        let source = indoc! {"
            ```text
            this line stays longer than five
            ```
        "};
        let rendered = Document::new()
            .verbatim(source)
            .render(RenderOptions::new(ColorMode::Never).width(5));
        assert_eq!(rendered, source);
    }

    #[test]
    fn verbatim_text_removes_terminal_bidi_and_line_controls() {
        let rendered = Document::new()
            .verbatim("\u{1b}]52;clipboard\u{7}\u{202e}\u{2028}\u{2029}\tvalue")
            .render(RenderOptions::new(ColorMode::Never).width(5));
        assert_eq!(rendered, "]52;clipboard\tvalue\n");
    }

    #[test]
    fn verbatim_text_preserves_other_unicode_formatting() {
        let source = "\u{600}\u{6dd}\u{70f}\u{110bd}\u{200b}\u{2060}\u{feff}\u{fff9}\u{1d173}\u{e0001}\u{200c}\u{200d}\u{ad}";
        let rendered = Document::new()
            .verbatim(source)
            .render(RenderOptions::new(ColorMode::Never).width(5));
        assert_eq!(rendered, format!("{source}\n"));
    }

    #[test]
    fn wrapped_stacked_labels_keep_indentation() {
        let table = Table::plain()
            .stacked_below(64, 1)
            .row(["one two three four five", "description"]);
        let rendered = Document::new()
            .table(table)
            .render(RenderOptions::new(ColorMode::Never).width(14));
        let expected = formatdoc! {"
            {label}one two
            {label}three four
            {label}five
            {description}descriptio
            {description}n
            ",
            label = "  ",
            description = "    ",
        };
        assert_eq!(rendered, expected);
    }
}

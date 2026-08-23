//! Render semantic documents without exposing the terminal engine.

use std::fmt::Write as _;

use comfy_table::presets::{NOTHING, UTF8_FULL_CONDENSED};
use comfy_table::{Cell, ContentArrangement, Table as EngineTable};
use unicode_width::UnicodeWidthStr;

use crate::color::ColorMode;
use crate::document::{Block, Document, Fields, Notice, NoticeLevel, Role, Section, Table, Text};
use crate::style::{ERROR, HEADING, MUTED, OPTION, SUCCESS, VALUE, WARNING, styled};

/// Deterministic document rendering options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    color: ColorMode,
    width: Option<u16>,
}

impl RenderOptions {
    /// Build options with automatic terminal width.
    #[must_use]
    pub const fn new(color: ColorMode) -> Self {
        Self { color, width: None }
    }

    /// Force an explicit width. Tests and redirected renderers should use this.
    #[must_use]
    pub const fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
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
            Block::Heading(text) => self.text_with_default(text, Role::Heading),
            Block::Paragraph(text) => self.wrap(&self.text(text), 0),
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
        let heading = self.text_with_default(section.title(), Role::Heading);
        let body = self.render(section.body());
        if body.is_empty() {
            heading
        } else {
            format!("{heading}\n{}", body.trim_end())
        }
    }

    fn fields(self, fields: &Fields) -> String {
        let mut table = self.engine_table();
        for (label, value) in fields.rows() {
            table.add_row([self.text(label), self.text(value)]);
        }
        format!("{table}")
    }

    fn table(self, table: &Table) -> String {
        if self.should_stack(table) {
            return self.stacked(table);
        }
        let mut engine = self.engine_table();
        if !table.headers().is_empty() {
            engine.set_header(
                table
                    .headers()
                    .iter()
                    .map(|header| Cell::new(self.text_with_default(header, Role::Heading))),
            );
        }
        for row in table.rows() {
            engine.add_row(row.iter().enumerate().map(|(index, cell)| {
                let value = if table.token_column_index() == Some(index) {
                    self.text_with_default(cell, Role::Token)
                } else {
                    self.text(cell)
                };
                Cell::new(value)
            }));
        }
        format!("{engine}")
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

    fn engine_table(self) -> EngineTable {
        let mut table = EngineTable::new();
        table
            .load_style(UTF8_FULL_CONDENSED)
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
        self.options
            .explicit_width()
            .or_else(crate::layout::terminal_width)
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

#[cfg(test)]
mod tests {
    use indoc::{formatdoc, indoc};

    use super::RenderOptions;
    use crate::color::ColorMode;
    use crate::document::{Document, Fields, Notice, NoticeLevel, Table, Text};

    #[test]
    fn colorless_document_is_deterministic() {
        let document = Document::new()
            .heading("status")
            .fields(Fields::new().row("pending", Text::plain("2")))
            .notice(Notice::new(NoticeLevel::Warning, "one stale row"));
        let rendered = document.render(RenderOptions::new(ColorMode::Never).width(60));
        let expected = indoc! {"
            status

            ┌─────────┬───┐
            │ pending ┆ 2 │
            └─────────┴───┘

            warning · one stale row
        "};
        assert_eq!(rendered, expected);
        assert!(!rendered.contains('\u{1b}'));
    }

    #[test]
    fn colored_document_has_ansi() {
        let document = Document::new().heading("status");
        let rendered = document.render(RenderOptions::new(ColorMode::Always).width(60));
        assert!(rendered.contains('\u{1b}'));
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

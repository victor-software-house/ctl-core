//! Render semantic documents through the ctl-layout widgets.
//!
//! A [`Document`] becomes a tree of widgets: [`Fields`] a record, [`Table`] a
//! table, a [`Section`] its heading over its body, a [`Notice`] and every
//! paragraph styled text, a rule a rule, and verbatim text its own lines. The
//! [`Theme`] turns each [`Role`] into a style on the way, so the widgets never
//! see a role. The widgets lay the document out as plain lines of styled
//! spans; escape sequences are added only when the lines are written.

use std::fmt;

use ctl_layout::{
    Column, Frame, Record, RecordStyle as RecordFrame, RowSeparation as RowRule,
    Rule as RuleWidget, Stack, Table as TableWidget,
};
use ctl_text::{
    Context, Line, Measurement, Overflow, Renderable, Span as StyledSpan, Style, Text as StyledText,
};

use crate::color::ColorMode;
use crate::document::{Block, Document, Fields, Notice, NoticeLevel, Role, Section, Table, Text};
use crate::theme::Theme;

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
/// Default environment variable for the operator's record style.
pub const DEFAULT_RECORD_STYLE_ENV: &str = "CTL_CORE_RECORD_STYLE";
/// Default ordered environment lookup for the record style.
pub const DEFAULT_RECORD_STYLE_ENVS: &[&str] = &[DEFAULT_RECORD_STYLE_ENV];
/// Default environment variable for the operator's list style.
pub const DEFAULT_LIST_STYLE_ENV: &str = "CTL_CORE_LIST_STYLE";
/// Default ordered environment lookup for the list style.
pub const DEFAULT_LIST_STYLE_ENVS: &[&str] = &[DEFAULT_LIST_STYLE_ENV];
/// Default environment variable for the operator's row separation.
pub const DEFAULT_ROW_SEPARATION_ENV: &str = "CTL_CORE_ROW_SEPARATION";
/// Default ordered environment lookup for the row separation.
pub const DEFAULT_ROW_SEPARATION_ENVS: &[&str] = &[DEFAULT_ROW_SEPARATION_ENV];

/// The width a document laid out at its natural width is given: wide enough
/// that nothing wraps.
const NATURAL_WIDTH: u16 = u16::MAX;

/// The width of a rule in a document laid out at its natural width, which a
/// rule, having no natural width of its own, would otherwise fill.
const NATURAL_RULE_WIDTH: u16 = 40;

/// The sequence that ends a styled run; see [`encode`].
const RESET: &str = "\u{1b}[0m";

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

/// A style an operator can name in an environment variable.
pub(crate) trait EnvChoice: Copy + 'static {
    /// Accepted values, in the order a warning lists them.
    const VALUES: &'static [(&'static str, Self)];
}

impl EnvChoice for RecordStyle {
    const VALUES: &'static [(&'static str, Self)] = &[
        ("keys-right", Self::KeysRight),
        ("keys-left", Self::KeysLeft),
        ("boxed", Self::Boxed),
    ];
}

impl EnvChoice for ListStyle {
    const VALUES: &'static [(&'static str, Self)] = &[
        ("grid", Self::Grid),
        ("header-rule", Self::HeaderRule),
        ("plain", Self::Plain),
    ];
}

impl EnvChoice for RowSeparation {
    const VALUES: &'static [(&'static str, Self)] = &[
        ("none", Self::None),
        ("rule", Self::Rule),
        ("blank", Self::Blank),
    ];
}

/// The first variable in `names` that holds an accepted value.
pub(crate) fn env_choice<T: EnvChoice>(
    names: &[&str],
    value: impl Fn(&str) -> Option<String>,
) -> Option<T> {
    names
        .iter()
        .filter_map(|name| value(name))
        .find_map(|raw| parse_choice(&raw))
}

/// One line per variable in `names` whose value is not accepted.
#[cfg(feature = "app")]
pub(crate) fn env_choice_warnings<T: EnvChoice>(
    names: &[&str],
    value: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| {
            let raw = value(name)?;
            parse_choice::<T>(&raw).is_none().then(|| {
                let accepted = T::VALUES
                    .iter()
                    .map(|(value, _)| *value)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}={raw} is ignored; use one of {accepted}")
            })
        })
        .collect()
}

/// A set, non-empty environment variable.
pub(crate) fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn parse_choice<T: EnvChoice>(raw: &str) -> Option<T> {
    T::VALUES
        .iter()
        .find(|(value, _)| *value == raw.trim())
        .map(|(_, choice)| *choice)
}

/// Deterministic document rendering options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    color: ColorMode,
    theme: Theme,
    width: Option<u16>,
    automatic_width_buffer: Option<u16>,
    automatic_width_buffer_envs: &'static [&'static str],
    minimum_automatic_width: u16,
    fallback_width: Option<u16>,
    record_style: Option<RecordStyle>,
    record_style_envs: &'static [&'static str],
    list_style: Option<ListStyle>,
    list_style_envs: &'static [&'static str],
    row_separation: Option<RowSeparation>,
    row_separation_envs: &'static [&'static str],
}

impl RenderOptions {
    /// Build options with automatic terminal width, a
    /// [`DEFAULT_COLUMN_BUFFER`]-column buffer, [`DEFAULT_COLUMN_BUFFER_ENV`]
    /// as the operator override, and [`DEFAULT_FALLBACK_WIDTH`] when no
    /// width is detected. Record style, list style, and row separation read
    /// [`DEFAULT_RECORD_STYLE_ENV`], [`DEFAULT_LIST_STYLE_ENV`], and
    /// [`DEFAULT_ROW_SEPARATION_ENV`] unless the owner sets them. Roles are
    /// drawn in [`Theme::default`].
    #[must_use]
    pub const fn new(color: ColorMode) -> Self {
        Self {
            color,
            theme: Theme::new(),
            width: None,
            automatic_width_buffer: None,
            automatic_width_buffer_envs: DEFAULT_COLUMN_BUFFER_ENVS,
            minimum_automatic_width: DEFAULT_MINIMUM_AUTOMATIC_WIDTH,
            fallback_width: Some(DEFAULT_FALLBACK_WIDTH),
            record_style: None,
            record_style_envs: DEFAULT_RECORD_STYLE_ENVS,
            list_style: None,
            list_style_envs: DEFAULT_LIST_STYLE_ENVS,
            row_separation: None,
            row_separation_envs: DEFAULT_ROW_SEPARATION_ENVS,
        }
    }

    /// Draw each [`Role`] in the style `theme` gives it, in place of
    /// [`Theme::default`]. Colour is still the colour mode's choice:
    /// [`ColorMode::Never`] writes no style.
    #[must_use]
    pub const fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// The theme roles are drawn in: the one [`Self::theme`] set, or
    /// [`Theme::default`].
    #[must_use]
    pub const fn active_theme(self) -> Theme {
        self.theme
    }

    /// Frame [`Fields`] records in `style`. This beats the environment.
    #[must_use]
    pub const fn record_style(mut self, style: RecordStyle) -> Self {
        self.record_style = Some(style);
        self
    }

    /// Frame [`Table`] lists in `style`. This beats the environment.
    #[must_use]
    pub const fn list_style(mut self, style: ListStyle) -> Self {
        self.list_style = Some(style);
        self
    }

    /// Separate [`Table`] rows with `separation`. This beats the environment.
    #[must_use]
    pub const fn row_separation(mut self, separation: RowSeparation) -> Self {
        self.row_separation = Some(separation);
        self
    }

    /// Replace the ordered environment names read for the record style.
    /// An empty slice disables lookup.
    #[must_use]
    pub const fn record_style_envs(mut self, names: &'static [&'static str]) -> Self {
        self.record_style_envs = names;
        self
    }

    /// Replace the ordered environment names read for the list style.
    /// An empty slice disables lookup.
    #[must_use]
    pub const fn list_style_envs(mut self, names: &'static [&'static str]) -> Self {
        self.list_style_envs = names;
        self
    }

    /// Replace the ordered environment names read for the row separation.
    /// An empty slice disables lookup.
    #[must_use]
    pub const fn row_separation_envs(mut self, names: &'static [&'static str]) -> Self {
        self.row_separation_envs = names;
        self
    }

    /// Record framing: the owner's choice, then the environment, then the
    /// default.
    #[must_use]
    pub fn record(self) -> RecordStyle {
        self.record_with(process_env)
    }

    /// List framing: the owner's choice, then the environment, then the
    /// default.
    #[must_use]
    pub fn list(self) -> ListStyle {
        self.list_with(process_env)
    }

    /// Row separation: the owner's choice, then the environment, then the
    /// default.
    #[must_use]
    pub fn separation(self) -> RowSeparation {
        self.separation_with(process_env)
    }

    fn record_with(self, value: impl Fn(&str) -> Option<String>) -> RecordStyle {
        self.record_style
            .or_else(|| env_choice(self.record_style_envs, value))
            .unwrap_or_default()
    }

    fn list_with(self, value: impl Fn(&str) -> Option<String>) -> ListStyle {
        self.list_style
            .or_else(|| env_choice(self.list_style_envs, value))
            .unwrap_or_default()
    }

    fn separation_with(self, value: impl Fn(&str) -> Option<String>) -> RowSeparation {
        self.row_separation
            .or_else(|| env_choice(self.row_separation_envs, value))
            .unwrap_or_default()
    }

    /// One line per style variable whose value is not accepted. A style the
    /// owner set never reads its variables, so it never warns.
    #[cfg(feature = "app")]
    pub(crate) fn style_env_warnings(self, value: impl Fn(&str) -> Option<String>) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.record_style.is_none() {
            warnings.extend(env_choice_warnings::<RecordStyle>(
                self.record_style_envs,
                &value,
            ));
        }
        if self.list_style.is_none() {
            warnings.extend(env_choice_warnings::<ListStyle>(
                self.list_style_envs,
                &value,
            ));
        }
        if self.row_separation.is_none() {
            warnings.extend(env_choice_warnings::<RowSeparation>(
                self.row_separation_envs,
                &value,
            ));
        }
        warnings
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

    /// The width policy as ctl-term's detection options: the explicit width,
    /// the buffer and its variables, the minimum, and the fallback.
    #[cfg(any(feature = "view", feature = "help"))]
    pub(crate) const fn detect_options(self) -> ctl_term::DetectOptions {
        let mut options = ctl_term::DetectOptions::new()
            .buffer_envs(self.automatic_width_buffer_envs)
            .minimum(self.minimum_automatic_width)
            .fallback_width(self.fallback_width);
        if let Some(width) = self.width {
            options = options.width(width);
        }
        if let Some(buffer) = self.automatic_width_buffer {
            options = options.buffer(buffer);
        }
        options
    }

    /// A writer for `stream` that decides colour and width for that stream:
    /// this colour mode, the process environment, and [`Self::detect_options`].
    #[cfg(any(feature = "view", feature = "help"))]
    pub(crate) fn console(self, stream: ctl_term::Stream) -> ctl_term::Console {
        ctl_term::Console::from_process_with(stream, self.color.into(), self.detect_options())
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
    ///
    /// The string is a pure function of the document and the options: no
    /// terminal is asked. The document lays out to the explicit width, else
    /// the fallback width, else its natural width. Styles are written only
    /// under [`ColorMode::Always`], as the theme gives them, at full colour
    /// depth; [`ColorMode::Auto`] has no stream to look at here and writes
    /// none. A [`View`](crate::View) or help, which know their stream, decide
    /// width and colour for it instead.
    #[must_use]
    pub fn render(self, document: &Document) -> String {
        let width = self
            .options
            .explicit_width()
            .or(self.options.fallback())
            .unwrap_or(NATURAL_WIDTH);
        let lines = self.widget(document).render(&Context::new(width));
        encode(&lines, self.options.color() == ColorMode::Always)
    }

    /// The document as one widget, with every role resolved through the
    /// theme. The caller picks the width and writes the lines.
    pub(crate) fn widget(self, document: &Document) -> Stack {
        document
            .blocks()
            .iter()
            .fold(Stack::new().spacing(1), |stack, block| {
                stack.push(Visible(self.block(block)))
            })
    }

    fn block(self, block: &Block) -> Box<dyn Renderable> {
        match block {
            Block::Heading(text) => Box::new(self.text(text, Some(Role::Heading))),
            Block::Paragraph(text) => Box::new(self.text(text, None)),
            Block::Verbatim(value) => Box::new(Verbatim::new(value)),
            Block::Fields(fields) => Box::new(self.fields(fields)),
            Block::Table(table) => self.table(table),
            Block::Section(section) => Box::new(self.section(section)),
            Block::Notice(notice) => Box::new(self.notice(notice)),
            Block::Rule(rule) => Box::new(Divider(
                rule.title().map_or_else(RuleWidget::new, |title| {
                    RuleWidget::titled(self.text(title, Some(Role::Heading)))
                }),
            )),
        }
    }

    /// The heading directly over the body, with no blank line between them
    /// and no indent, so a section's tables line up with the document's.
    fn section(self, section: &Section) -> Stack {
        Stack::new()
            .push(Visible(Box::new(
                self.text(section.title(), Some(Role::Heading)),
            )))
            .push(Visible(Box::new(self.widget(section.body()))))
    }

    fn fields(self, fields: &Fields) -> Record {
        let style = match self.options.record() {
            RecordStyle::Boxed => RecordFrame::Boxed,
            RecordStyle::KeysRight => RecordFrame::KeysRight,
            RecordStyle::KeysLeft => RecordFrame::KeysLeft,
        };
        fields
            .rows()
            .iter()
            .fold(Record::new().style(style), |record, (label, value)| {
                record.row(self.text(label, None), self.text(value, None))
            })
    }

    fn table(self, table: &Table) -> Box<dyn Renderable> {
        let count = table
            .rows()
            .iter()
            .map(Vec::len)
            .fold(table.headers().len(), usize::max);
        let columns = (0..count)
            .map(|index| self.column(table, index))
            .collect::<Vec<_>>();
        let frame = match self.options.list() {
            ListStyle::Grid => Frame::Grid,
            ListStyle::HeaderRule => Frame::HeaderRule,
            ListStyle::Plain => Frame::Plain,
        };
        let separation = match self.options.separation() {
            RowSeparation::None => RowRule::None,
            RowSeparation::Rule => RowRule::Rule,
            RowSeparation::Blank => RowRule::Blank,
        };
        let grid = TableWidget::new(columns.iter().cloned())
            .frame(frame)
            .row_separation(separation)
            .rows(table.rows().iter().map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(index, cell)| self.text(cell, cell_role(table, index)))
                    .collect::<Vec<_>>()
            }));
        let Some(policy) = table.stacked() else {
            return Box::new(grid);
        };
        let stacked = TableWidget::new(columns)
            .stacked_below(policy.width(), policy.label_columns())
            .rows(table.rows().iter().map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(index, cell)| {
                        let label = index < policy.label_columns();
                        self.text(cell, label.then_some(Role::Token))
                    })
                    .collect::<Vec<_>>()
            }));
        Box::new(Stackable {
            threshold: policy.width(),
            grid,
            stacked,
        })
    }

    fn column(self, table: &Table, index: usize) -> Column {
        let column = if table.headers().is_empty() {
            Column::untitled()
        } else {
            Column::styled(
                table
                    .headers()
                    .get(index)
                    .map_or_else(StyledText::new, |header| {
                        self.text(header, Some(Role::Heading))
                    }),
            )
        };
        let column = if table.ellipsis_columns().contains(&index) {
            column.overflow(Overflow::Ellipsis)
        } else {
            column
        };
        if table.numeric_columns().contains(&index) {
            column.numeric()
        } else {
            column
        }
    }

    fn notice(self, notice: &Notice) -> StyledText {
        let theme = self.options.active_theme();
        let label = match notice.level() {
            NoticeLevel::Success => "success",
            NoticeLevel::Warning => "warning",
            NoticeLevel::Error => "error",
        };
        let mut text = StyledText::styled(label, theme.notice(notice.level()));
        if let Some(code) = notice.code_value() {
            text.push(" · ", Style::new());
            text.push(code, theme.role(Role::Muted));
        }
        text.push(" · ", Style::new());
        self.push_spans(&mut text, notice.message(), None);
        text
    }

    /// `text` with each span in its role's style. A plain span takes
    /// `default` when one is given, so a heading or a token column styles
    /// the text a consumer left plain.
    fn text(self, text: &Text, default: Option<Role>) -> StyledText {
        let mut styled = StyledText::new();
        self.push_spans(&mut styled, text, default);
        styled
    }

    fn push_spans(self, target: &mut StyledText, text: &Text, default: Option<Role>) {
        let theme = self.options.active_theme();
        for span in text.spans() {
            let role = match (span.role(), default) {
                (Role::Plain, Some(default)) => default,
                (role, _) => role,
            };
            target.push(span.value(), theme.role(role));
        }
    }
}

/// The role a table cell's plain spans take: a token column's, then an id
/// column's, then none.
fn cell_role(table: &Table, index: usize) -> Option<Role> {
    if table.token_column_index() == Some(index) {
        Some(Role::Token)
    } else if table.id_column_index() == Some(index) {
        Some(Role::Id)
    } else {
        None
    }
}

impl Document {
    /// Render with explicit semantic options; see [`Renderer::render`].
    #[must_use]
    pub fn render(&self, options: RenderOptions) -> String {
        Renderer::new(options).render(self)
    }
}

/// `lines` as text, each line ended by a newline, with styles written only
/// when `color` is set.
///
/// This writes the bytes `ctl_term::Encoder` writes at full colour depth: a
/// style sequence where the style changes, a reset before a change away from
/// a style, and a reset at the end of every line that wrote a style. A
/// [`Document::render`] string has no stream and so no depth to fit; the
/// `render` tier stays free of the terminal crate. The parity test checks
/// that this and the view's writer give the same bytes.
fn encode(lines: &[Line], color: bool) -> String {
    let mut out = String::new();
    for line in lines {
        let mut current = Style::new();
        let mut wrote_style = false;
        for span in line.spans() {
            let style = if color { span.style() } else { Style::new() };
            if style != current {
                if !current.is_plain() {
                    out.push_str(RESET);
                }
                if !style.is_plain() {
                    out.push_str(&style.render().to_string());
                    wrote_style = true;
                }
                current = style;
            }
            out.push_str(span.text());
        }
        if wrote_style {
            out.push_str(RESET);
        }
        out.push('\n');
    }
    out
}

/// A block that draws nothing when every line it renders is empty, so an
/// empty paragraph or an empty record leaves no blank lines in the stack.
struct Visible(Box<dyn Renderable>);

impl fmt::Debug for Visible {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Visible").finish_non_exhaustive()
    }
}

impl Renderable for Visible {
    fn measure(&self, context: &Context) -> Measurement {
        self.0.measure(context)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        let lines = self.0.render(context);
        if lines.iter().all(Line::is_empty) {
            Vec::new()
        } else {
            lines
        }
    }
}

/// Preformatted lines, drawn as given at any width.
///
/// The unsafe characters go, newline aside, and so do the newlines at the
/// end. A tab expands to the next multiple of 8 cells from the start of its
/// line, since a span holds no tab. Trailing spaces stay.
#[derive(Debug)]
struct Verbatim {
    lines: Vec<Line>,
}

impl Verbatim {
    fn new(value: &str) -> Self {
        let kept = value
            .chars()
            .filter(|character| {
                matches!(*character, '\n' | '\t') || !ctl_text::verbatim::is_unsafe(*character)
            })
            .collect::<String>();
        let kept = kept.trim_end_matches('\n');
        let lines = if kept.is_empty() {
            Vec::new()
        } else {
            kept.split('\n')
                .map(|line| Line::from(StyledSpan::new(line, Style::new())))
                .collect()
        };
        Self { lines }
    }
}

impl Renderable for Verbatim {
    fn measure(&self, _: &Context) -> Measurement {
        Measurement::exact(self.lines.iter().map(Line::width).max().unwrap_or(0))
    }

    fn render(&self, _: &Context) -> Vec<Line> {
        self.lines.clone()
    }
}

/// A rule across the width, or across [`NATURAL_RULE_WIDTH`] cells in a
/// document laid out at its natural width.
#[derive(Debug)]
struct Divider(RuleWidget);

impl Divider {
    fn context(context: Context) -> Context {
        if context.width() == NATURAL_WIDTH {
            context.with_width(NATURAL_RULE_WIDTH)
        } else {
            context
        }
    }
}

impl Renderable for Divider {
    fn measure(&self, context: &Context) -> Measurement {
        self.0.measure(&Self::context(*context))
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.0.render(&Self::context(*context))
    }
}

/// A table that stacks its rows below a width.
///
/// The two forms style their cells differently: a stacked row's label cells
/// take the token style, as help's flags do, while the grid styles only its
/// token and id columns. So each form is its own table, and the width picks
/// one.
#[derive(Debug)]
struct Stackable {
    threshold: u16,
    grid: TableWidget,
    stacked: TableWidget,
}

impl Stackable {
    fn pick(&self, context: Context) -> &TableWidget {
        if context.width() < self.threshold {
            &self.stacked
        } else {
            &self.grid
        }
    }
}

impl Renderable for Stackable {
    fn measure(&self, context: &Context) -> Measurement {
        self.pick(*context).measure(context)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.pick(*context).render(context)
    }
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

    fn operator_env(name: &str) -> Option<String> {
        match name {
            "CTL_CORE_RECORD_STYLE" => Some("boxed"),
            "CTL_CORE_LIST_STYLE" => Some("header-rule"),
            "CTL_CORE_ROW_SEPARATION" => Some("blank"),
            _ => None,
        }
        .map(str::to_owned)
    }

    #[test]
    fn the_environment_picks_each_style_the_owner_left_open() {
        let open = RenderOptions::new(ColorMode::Never);
        assert_eq!(open.record_with(operator_env), RecordStyle::Boxed);
        assert_eq!(open.list_with(operator_env), ListStyle::HeaderRule);
        assert_eq!(open.separation_with(operator_env), RowSeparation::Blank);

        let owned = open
            .record_style(RecordStyle::KeysLeft)
            .list_style(ListStyle::Plain)
            .row_separation(RowSeparation::Rule);
        assert_eq!(owned.record_with(operator_env), RecordStyle::KeysLeft);
        assert_eq!(owned.list_with(operator_env), ListStyle::Plain);
        assert_eq!(owned.separation_with(operator_env), RowSeparation::Rule);
    }

    #[test]
    fn style_env_names_can_be_replaced_or_disabled() {
        let aliased = |name: &str| (name == "TOY_RECORD").then(|| "keys-left".to_owned());
        let replaced = RenderOptions::new(ColorMode::Never)
            .record_style_envs(&["TOY_RECORD", "CTL_CORE_RECORD_STYLE"]);
        assert_eq!(replaced.record_with(aliased), RecordStyle::KeysLeft);

        let disabled = RenderOptions::new(ColorMode::Never)
            .record_style_envs(&[])
            .list_style_envs(&[])
            .row_separation_envs(&[]);
        assert_eq!(disabled.record_with(operator_env), RecordStyle::KeysRight);
        assert_eq!(disabled.list_with(operator_env), ListStyle::Grid);
        assert_eq!(disabled.separation_with(operator_env), RowSeparation::None);
    }

    #[cfg(feature = "app")]
    #[test]
    fn an_unknown_style_value_is_ignored_and_the_warning_names_the_choices() {
        let typo = |name: &str| (name == "CTL_CORE_RECORD_STYLE").then(|| "boxy".to_owned());
        let options = RenderOptions::new(ColorMode::Never);
        assert_eq!(options.record_with(typo), RecordStyle::KeysRight);
        assert_eq!(
            options.style_env_warnings(typo),
            ["CTL_CORE_RECORD_STYLE=boxy is ignored; use one of keys-right, keys-left, boxed"]
        );
        assert_eq!(
            options
                .record_style(RecordStyle::Boxed)
                .style_env_warnings(typo),
            Vec::<String>::new()
        );
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
    fn long_words_run_past_the_width_instead_of_splitting() {
        let url = "https://example.test/releases/tag/pkg@1.2.3";
        let rendered = Document::new()
            .fields(Fields::new().row("release", Text::plain(url)))
            .paragraph(Text::plain(format!("see {url}")))
            .render(RenderOptions::new(ColorMode::Always).width(30));
        assert_eq!(rendered.matches(url).count(), 2, "{rendered}");
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
        assert_eq!(rendered, "]52;clipboard   value\n");
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
            {description}description
            ",
            label = "  ",
            description = "    ",
        };
        assert_eq!(rendered, expected);
    }
}

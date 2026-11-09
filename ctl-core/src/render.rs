//! Render semantic documents through the ctl-layout widgets.
//!
//! A [`Document`] becomes a tree of widgets, one per block:
//!
//! - [`Fields`] becomes a record, and [`Table`] a table.
//! - A [`Section`] becomes its heading over its body.
//! - A [`Notice`] and every paragraph become styled text.
//! - A rule stays a rule, and verbatim text keeps its own lines.
//!
//! The [`Theme`] turns each [`Role`] into a style on the way, so the widgets
//! never see a role. The widgets lay the document out as plain lines of styled
//! spans. Escape sequences are added only when ctl-term's encoder writes the
//! lines.

use ctl_layout::{
    Breakpoint, Column, Definitions, Record, Rule as RuleWidget, Stack, Table as TableWidget,
};
pub use ctl_layout::{Frame, RecordStyle, RowSeparation};
use ctl_term::{ColorDepth, Encoder};
use ctl_text::unsafe_chars::{Keep, strip_unsafe};
use ctl_text::{
    Context, Line, Measurement, Overflow, Renderable, Span as StyledSpan, Style, Text as StyledText,
};

use crate::color::ColorMode;
use crate::document::{Block, Document, Fields, Notice, NoticeLevel, Role, Section, Table, Text};
use crate::theme::Theme;

/// Default cells reserved from an automatically detected terminal width.
#[cfg(any(feature = "view", feature = "help"))]
pub const DEFAULT_COLUMN_BUFFER: u16 = ctl_term::DEFAULT_COLUMN_BUFFER;
/// The environment variable that sets the automatic-width buffer, in cells,
/// when the CLI has not: `CTL_CORE_COLUMN_BUFFER`.
#[cfg(any(feature = "view", feature = "help"))]
pub const DEFAULT_COLUMN_BUFFER_ENV: &str = "CTL_CORE_COLUMN_BUFFER";
/// The ordered list of variables read for the automatic-width buffer, holding
/// only [`DEFAULT_COLUMN_BUFFER_ENV`].
#[cfg(any(feature = "view", feature = "help"))]
pub const DEFAULT_COLUMN_BUFFER_ENVS: &[&str] = &[DEFAULT_COLUMN_BUFFER_ENV];
/// Default floor for an automatically detected effective width.
#[cfg(any(feature = "view", feature = "help"))]
pub const DEFAULT_MINIMUM_AUTOMATIC_WIDTH: u16 = ctl_term::DEFAULT_MINIMUM_WIDTH;
/// Width used when neither the terminal nor `COLUMNS` gives one.
pub const DEFAULT_FALLBACK_WIDTH: u16 = ctl_term::DEFAULT_FALLBACK_WIDTH;
/// The environment variable that sets the record style when the CLI has not:
/// `CTL_CORE_RECORD_STYLE`. Its values are the [`RecordStyle`] names
/// `keys-right`, `keys-left`, and `boxed`.
pub const DEFAULT_RECORD_STYLE_ENV: &str = "CTL_CORE_RECORD_STYLE";
/// The ordered list of variables read for the record style, holding only
/// [`DEFAULT_RECORD_STYLE_ENV`].
pub const DEFAULT_RECORD_STYLE_ENVS: &[&str] = &[DEFAULT_RECORD_STYLE_ENV];
/// The environment variable that sets the table frame of a list when the CLI
/// has not: `CTL_CORE_LIST_STYLE`. Its values are the [`Frame`] names `grid`,
/// `header-rule`, and `plain`.
pub const DEFAULT_LIST_STYLE_ENV: &str = "CTL_CORE_LIST_STYLE";
/// The ordered list of variables read for the table frame of a list, holding
/// only [`DEFAULT_LIST_STYLE_ENV`].
pub const DEFAULT_LIST_STYLE_ENVS: &[&str] = &[DEFAULT_LIST_STYLE_ENV];
/// The environment variable that sets the row separation when the CLI has not:
/// `CTL_CORE_ROW_SEPARATION`. Its values are the [`RowSeparation`] names
/// `none`, `rule`, and `blank`.
pub const DEFAULT_ROW_SEPARATION_ENV: &str = "CTL_CORE_ROW_SEPARATION";
/// The ordered list of variables read for the row separation, holding only
/// [`DEFAULT_ROW_SEPARATION_ENV`].
pub const DEFAULT_ROW_SEPARATION_ENVS: &[&str] = &[DEFAULT_ROW_SEPARATION_ENV];

/// The width of a rule in a document laid out at its natural width. A rule has
/// no natural width of its own, so it would otherwise fill that width.
const NATURAL_RULE_WIDTH: u16 = 40;

/// A style a user can name in an environment variable.
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

impl EnvChoice for Frame {
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

/// How a document renders: colour mode, width, theme, and styles.
///
/// A setter takes the name of its setting and returns the options changed.
/// The options have no getters, which would need the same names. A method that
/// reads the environment starts with `resolved_`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    color: ColorMode,
    theme: Theme,
    width: Option<u16>,
    #[cfg(any(feature = "view", feature = "help"))]
    automatic_width_buffer: Option<u16>,
    #[cfg(any(feature = "view", feature = "help"))]
    automatic_width_buffer_envs: &'static [&'static str],
    #[cfg(any(feature = "view", feature = "help"))]
    minimum_automatic_width: u16,
    fallback_width: Option<u16>,
    record_style: Option<RecordStyle>,
    record_style_envs: &'static [&'static str],
    frame: Option<Frame>,
    frame_envs: &'static [&'static str],
    row_separation: Option<RowSeparation>,
    row_separation_envs: &'static [&'static str],
}

impl RenderOptions {
    /// Build options with [`DEFAULT_FALLBACK_WIDTH`] when no width is
    /// detected. Record style, table frame, and row separation read
    /// [`DEFAULT_RECORD_STYLE_ENV`], [`DEFAULT_LIST_STYLE_ENV`], and
    /// [`DEFAULT_ROW_SEPARATION_ENV`] unless the CLI sets them. Roles are
    /// drawn in [`Theme::default`].
    ///
    /// With `view` or `help`, the width is detected for the stream written,
    /// with a buffer of `DEFAULT_COLUMN_BUFFER` cells and
    /// `DEFAULT_COLUMN_BUFFER_ENV` as the user's override.
    #[must_use]
    pub const fn new(color: ColorMode) -> Self {
        Self {
            color,
            theme: Theme::new(),
            width: None,
            #[cfg(any(feature = "view", feature = "help"))]
            automatic_width_buffer: None,
            #[cfg(any(feature = "view", feature = "help"))]
            automatic_width_buffer_envs: DEFAULT_COLUMN_BUFFER_ENVS,
            #[cfg(any(feature = "view", feature = "help"))]
            minimum_automatic_width: DEFAULT_MINIMUM_AUTOMATIC_WIDTH,
            fallback_width: Some(DEFAULT_FALLBACK_WIDTH),
            record_style: None,
            record_style_envs: DEFAULT_RECORD_STYLE_ENVS,
            frame: None,
            frame_envs: DEFAULT_LIST_STYLE_ENVS,
            row_separation: None,
            row_separation_envs: DEFAULT_ROW_SEPARATION_ENVS,
        }
    }

    /// Write colour as `color` says, in place of the mode given to
    /// [`Self::new`].
    #[must_use]
    pub const fn color(mut self, color: ColorMode) -> Self {
        self.color = color;
        self
    }

    /// Draw each [`Role`] in the style `theme` gives it, in place of
    /// [`Theme::default`]. Colour is still the colour mode's choice:
    /// [`ColorMode::Never`] writes no style.
    #[must_use]
    pub const fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Frame [`Fields`] records in `style`. This beats the environment.
    #[must_use]
    pub const fn record_style(mut self, style: RecordStyle) -> Self {
        self.record_style = Some(style);
        self
    }

    /// Set the table frame of [`Table`] lists to `frame`. This beats the
    /// environment.
    #[must_use]
    pub const fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
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

    /// Replace the ordered environment names read for the table frame.
    /// An empty slice disables lookup.
    #[must_use]
    pub const fn frame_envs(mut self, names: &'static [&'static str]) -> Self {
        self.frame_envs = names;
        self
    }

    /// Replace the ordered environment names read for the row separation.
    /// An empty slice disables lookup.
    #[must_use]
    pub const fn row_separation_envs(mut self, names: &'static [&'static str]) -> Self {
        self.row_separation_envs = names;
        self
    }

    /// Force an explicit width. Use it in tests and wherever no terminal
    /// decides the width.
    #[must_use]
    pub const fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Override the automatic-width buffer. Zero explicitly disables it.
    /// Explicit [`Self::width`] remains exact regardless of this value.
    #[cfg(any(feature = "view", feature = "help"))]
    #[must_use]
    pub const fn automatic_width_buffer(mut self, cells: u16) -> Self {
        self.automatic_width_buffer = Some(cells);
        self
    }

    /// Replace the ordered environment names that set the buffer.
    /// Pass aliases in precedence order or an empty slice to disable lookup.
    #[cfg(any(feature = "view", feature = "help"))]
    #[must_use]
    pub const fn automatic_width_buffer_envs(mut self, names: &'static [&'static str]) -> Self {
        self.automatic_width_buffer_envs = names;
        self
    }

    /// Set the floor for automatically detected effective widths.
    #[cfg(any(feature = "view", feature = "help"))]
    #[must_use]
    pub const fn minimum_automatic_width(mut self, cells: u16) -> Self {
        self.minimum_automatic_width = cells;
        self
    }

    /// Lay out to `width` when neither the terminal nor `COLUMNS` gives one.
    /// `None` renders tables at their natural width instead. This does not
    /// touch the minimum automatic width, which applies only to a detected
    /// width.
    #[must_use]
    pub const fn fallback_width(mut self, width: Option<u16>) -> Self {
        self.fallback_width = width;
        self
    }

    /// Take the theme, record style, table frame, row separation, and the
    /// variables they read from `styles`. Colour and width stay as they are.
    #[cfg(feature = "view")]
    #[must_use]
    pub(crate) const fn styles(mut self, styles: Self) -> Self {
        self.theme = styles.theme;
        self.record_style = styles.record_style;
        self.record_style_envs = styles.record_style_envs;
        self.frame = styles.frame;
        self.frame_envs = styles.frame_envs;
        self.row_separation = styles.row_separation;
        self.row_separation_envs = styles.row_separation_envs;
        self
    }

    /// Record framing: the CLI's choice, then the environment, then the
    /// default. It reads the process environment.
    #[must_use]
    pub fn resolved_record_style(self) -> RecordStyle {
        self.record_style_with(process_env)
    }

    /// Table frame: the CLI's choice, then the environment, then the
    /// default. It reads the process environment.
    #[must_use]
    pub fn resolved_frame(self) -> Frame {
        self.frame_with(process_env)
    }

    /// Row separation: the CLI's choice, then the environment, then the
    /// default. It reads the process environment.
    #[must_use]
    pub fn resolved_row_separation(self) -> RowSeparation {
        self.row_separation_with(process_env)
    }

    fn record_style_with(self, value: impl Fn(&str) -> Option<String>) -> RecordStyle {
        self.record_style
            .or_else(|| env_choice(self.record_style_envs, value))
            .unwrap_or_default()
    }

    fn frame_with(self, value: impl Fn(&str) -> Option<String>) -> Frame {
        self.frame
            .or_else(|| env_choice(self.frame_envs, value))
            .unwrap_or_default()
    }

    fn row_separation_with(self, value: impl Fn(&str) -> Option<String>) -> RowSeparation {
        self.row_separation
            .or_else(|| env_choice(self.row_separation_envs, value))
            .unwrap_or_default()
    }

    /// One line per style variable whose value is not accepted. A style the
    /// CLI set never reads its variables, so it never warns.
    #[cfg(feature = "app")]
    pub(crate) fn style_env_warnings(self, value: impl Fn(&str) -> Option<String>) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.record_style.is_none() {
            warnings.extend(env_choice_warnings::<RecordStyle>(
                self.record_style_envs,
                &value,
            ));
        }
        if self.frame.is_none() {
            warnings.extend(env_choice_warnings::<Frame>(self.frame_envs, &value));
        }
        if self.row_separation.is_none() {
            warnings.extend(env_choice_warnings::<RowSeparation>(
                self.row_separation_envs,
                &value,
            ));
        }
        warnings
    }

    /// The width policy as ctl-term's detection options: the explicit width,
    /// the buffer and its variables, the minimum, and the fallback.
    #[cfg(any(feature = "view", feature = "help"))]
    const fn detect_options(self) -> ctl_term::DetectOptions {
        let mut options = ctl_term::DetectOptions::new()
            .column_buffer_envs(self.automatic_width_buffer_envs)
            .minimum_width(self.minimum_automatic_width)
            .fallback_width(self.fallback_width);
        if let Some(width) = self.width {
            options = options.width(width);
        }
        if let Some(buffer) = self.automatic_width_buffer {
            options = options.column_buffer(buffer);
        }
        options
    }

    /// A writer for `stream` that decides colour and width for that stream:
    /// this colour mode, the process environment, and [`Self::detect_options`].
    #[cfg(any(feature = "view", feature = "help"))]
    fn console(self, stream: ctl_term::Stream) -> ctl_term::Console {
        ctl_term::Console::from_process(stream, self.color.into(), self.detect_options())
    }

    /// `document` as text in the colour and at the width decided for `stream`.
    #[cfg(feature = "view")]
    pub(crate) fn render_for(self, stream: ctl_term::Stream, document: &Document) -> String {
        self.console(stream)
            .render(&Renderer::new(self).widget(document))
    }

    /// Write `document` to `stream` in the colour and at the width decided for
    /// it.
    ///
    /// The error keeps the kind of the failed write, so a closed pipe is still
    /// [`std::io::ErrorKind::BrokenPipe`]. It goes through ctl-term's
    /// conversion rather than a boxed wrapper, which would keep the wrapper's
    /// `Debug` code in every binary that prints help.
    #[cfg(feature = "help")]
    pub(crate) fn print_for(
        self,
        stream: ctl_term::Stream,
        document: &Document,
    ) -> std::io::Result<()> {
        self.console(stream)
            .print(&Renderer::new(self).widget(document))
            .map_err(std::io::Error::from)
    }
}

/// Semantic document renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Renderer {
    options: RenderOptions,
}

impl Renderer {
    /// Build a renderer.
    pub(crate) const fn new(options: RenderOptions) -> Self {
        Self { options }
    }

    /// Render one document to a newline-terminated string, as
    /// [`Document::render`] says.
    pub(crate) fn render(self, document: &Document) -> String {
        let context = self
            .options
            .width
            .or(self.options.fallback_width)
            .map_or_else(Context::unbounded, Context::new);
        let lines = self.widget(document).render(&context);
        let depth = (self.options.color == ColorMode::Always).then_some(ColorDepth::TrueColor);
        Encoder::new(depth).encode(&lines)
    }

    /// The document as one widget, with every role resolved through the
    /// theme. The caller picks the width and writes the lines.
    pub(crate) fn widget(self, document: &Document) -> Stack<'static> {
        document
            .blocks()
            .iter()
            .fold(Stack::new().spacing(1), |stack, block| {
                stack.push(self.block(block))
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

    /// The heading directly over the body. There is no blank line between them
    /// and no indent, so a section's tables line up with the document's.
    fn section(self, section: &Section) -> Stack<'static> {
        Stack::new()
            .push(self.text(section.title(), Some(Role::Heading)))
            .push(self.widget(section.body()))
    }

    fn fields(self, fields: &Fields) -> Record {
        fields.rows().iter().fold(
            Record::new().style(self.options.resolved_record_style()),
            |record, (label, value)| record.row(self.text(label, None), self.text(value, None)),
        )
    }

    fn table(self, table: &Table) -> Box<dyn Renderable> {
        let count = table
            .rows()
            .iter()
            .map(Vec::len)
            .fold(table.headers().len(), usize::max);
        let grid = TableWidget::new((0..count).map(|index| self.column(table, index)))
            .frame(self.options.resolved_frame())
            .row_separation(self.options.resolved_row_separation())
            .rows(table.rows().iter().map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(index, cell)| self.text(cell, cell_role(table, index)))
                    .collect::<Vec<_>>()
            }));
        let Some(policy) = table.stacked() else {
            return Box::new(grid);
        };
        let definitions = table
            .rows()
            .iter()
            .fold(Definitions::new(), |definitions, row| {
                let (labels, description) = row.split_at(row.len().min(policy.label_columns()));
                definitions.entry(
                    labels.iter().map(|cell| self.text(cell, Some(Role::Token))),
                    description.iter().map(|cell| self.text(cell, None)),
                )
            });
        Box::new(Breakpoint::new(policy.width(), definitions, grid))
    }

    fn column(self, table: &Table, index: usize) -> Column {
        let column = if table.headers().is_empty() {
            Column::untitled()
        } else {
            Column::new(
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
        let theme = self.options.theme;
        let label = match notice.level() {
            NoticeLevel::Success => "success",
            NoticeLevel::Warning => "warning",
            NoticeLevel::Error => "error",
        };
        let separator = theme.role_style(Role::Plain);
        let mut text = StyledText::styled(label, theme.label_style(notice.level()));
        if let Some(code) = notice.code_value() {
            text.push(" · ", separator);
            text.push(code, theme.role_style(Role::Muted));
        }
        text.push(" · ", separator);
        self.push_spans(&mut text, notice.message(), None);
        text
    }

    /// `text` with each span in its role's style. A plain span takes
    /// `default` when one is given, so a heading or a token column styles
    /// the text a CLI left plain.
    fn text(self, text: &Text, default: Option<Role>) -> StyledText {
        let mut styled = StyledText::new();
        self.push_spans(&mut styled, text, default);
        styled
    }

    fn push_spans(self, target: &mut StyledText, text: &Text, default: Option<Role>) {
        let theme = self.options.theme;
        for span in text.spans() {
            let role = match (span.role(), default) {
                (Role::Plain, Some(default)) => default,
                (role, _) => role,
            };
            target.push(span.value(), theme.role_style(role));
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
    /// Render with explicit semantic options; see [`RenderOptions`].
    ///
    /// The string is a pure function of the document and the options: no
    /// terminal is asked. The document lays out to the explicit width, else
    /// the fallback width, else its natural width.
    ///
    /// Styles are written only under [`ColorMode::Always`], as the theme gives
    /// them, at full colour depth. [`ColorMode::Auto`] has no stream to look
    /// at here and writes none. A [`View`](crate::View) and help write to a
    /// stream, so they decide width and colour for it instead.
    #[must_use]
    pub fn render(&self, options: RenderOptions) -> String {
        Renderer::new(options).render(self)
    }
}

/// Preformatted lines, drawn as given at any width.
///
/// The unsafe characters go, newline and tab aside, and so do the newlines at
/// the end. A tab expands to the next multiple of 8 cells from the start of
/// its line, since a span holds no tab. Trailing spaces stay.
#[derive(Debug)]
struct Verbatim {
    lines: Vec<Line>,
}

impl Verbatim {
    fn new(value: &str) -> Self {
        let kept = strip_unsafe(value, Keep::NewlineAndTab);
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

/// A rule across the width, or across [`NATURAL_RULE_WIDTH`] cells in an
/// unbounded context, which [`Context::is_unbounded`] reports. A rule has no
/// natural width, so it would otherwise fill the largest width.
#[derive(Debug)]
struct Divider(RuleWidget);

impl Divider {
    fn context(context: &Context) -> Context {
        if context.is_unbounded() {
            context.at_width(NATURAL_RULE_WIDTH)
        } else {
            context.clone()
        }
    }
}

impl Renderable for Divider {
    fn measure(&self, context: &Context) -> Measurement {
        self.0.measure(&Self::context(context))
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        self.0.render(&Self::context(context))
    }
}

#[cfg(test)]
mod tests {
    use ctl_text::Style;
    use ctl_text::anstyle::Effects;
    use indoc::{formatdoc, indoc};

    use super::{Frame, RecordStyle, RenderOptions, RowSeparation};
    use crate::color::ColorMode;
    use crate::document::{Document, Fields, Notice, NoticeLevel, Role, Section, Table, Text};
    use crate::theme::Theme;

    #[cfg(any(feature = "view", feature = "help"))]
    #[test]
    fn automatic_width_policy_is_explicit_and_overridable() {
        let defaults = RenderOptions::new(ColorMode::Never);
        assert_eq!(super::DEFAULT_COLUMN_BUFFER, 2);
        assert_eq!(defaults.automatic_width_buffer, None);
        assert_eq!(
            defaults.minimum_automatic_width,
            super::DEFAULT_MINIMUM_AUTOMATIC_WIDTH
        );

        let configured = defaults
            .automatic_width_buffer(0)
            .automatic_width_buffer_envs(&["APP_COLUMNS_BUFFER", "LEGACY_BUFFER"])
            .minimum_automatic_width(8);
        assert_eq!(configured.automatic_width_buffer, Some(0));
        assert_eq!(
            configured.automatic_width_buffer_envs,
            ["APP_COLUMNS_BUFFER", "LEGACY_BUFFER"]
        );
        assert_eq!(configured.minimum_automatic_width, 8);
    }

    #[cfg(any(feature = "view", feature = "help"))]
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
            .row(["task-014", "Let a ledger declare row separation"])
            .row(["task-015", "Write a ledger atomically"])
            .row(["task-016", "Name the next startable row"])
    }

    #[test]
    fn borderless_record_right_aligns_keys() {
        let rendered = Document::new()
            .fields(
                Fields::new()
                    .row("ledger", Text::plain("tasks.yaml"))
                    .row("active", Text::plain("task-014")),
            )
            .render(
                RenderOptions::new(ColorMode::Never)
                    .width(80)
                    .record_style(RecordStyle::KeysRight),
            );
        let expected = indoc! {"
            ledger  tasks.yaml
            active  task-014
        "};
        assert_eq!(rendered, expected);
    }

    #[test]
    fn left_aligned_record_pads_short_keys_on_the_right() {
        let rendered = Document::new()
            .fields(
                Fields::new()
                    .row("id", Text::plain("task-014"))
                    .row("outcome", Text::plain("done")),
            )
            .render(
                RenderOptions::new(ColorMode::Never)
                    .width(80)
                    .record_style(RecordStyle::KeysLeft),
            );
        let expected = indoc! {"
            id       task-014
            outcome  done
        "};
        assert_eq!(rendered, expected);
    }

    #[test]
    fn header_rule_draws_one_rule_and_no_verticals() {
        let rendered = Document::new().table(queue()).render(
            RenderOptions::new(ColorMode::Never)
                .width(80)
                .frame(Frame::HeaderRule),
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
                .frame(Frame::Plain)
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
                .frame(Frame::Plain)
                .row_separation(RowSeparation::Blank),
        );
        let lines = rendered.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 6, "{rendered}");
        assert!(lines[2].is_empty() && lines[4].is_empty(), "{rendered}");
    }

    #[test]
    fn defaults_are_the_operator_picks() {
        let options = RenderOptions::new(ColorMode::Never);
        assert_eq!(options.resolved_record_style(), RecordStyle::KeysRight);
        assert_eq!(options.resolved_frame(), Frame::Grid);
        assert_eq!(options.resolved_row_separation(), RowSeparation::None);
        assert_eq!(options.fallback_width, Some(80));
        assert_eq!(options.fallback_width(None).fallback_width, None);
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
        assert_eq!(open.record_style_with(operator_env), RecordStyle::Boxed);
        assert_eq!(open.frame_with(operator_env), Frame::HeaderRule);
        assert_eq!(open.row_separation_with(operator_env), RowSeparation::Blank);

        let owned = open
            .record_style(RecordStyle::KeysLeft)
            .frame(Frame::Plain)
            .row_separation(RowSeparation::Rule);
        assert_eq!(owned.record_style_with(operator_env), RecordStyle::KeysLeft);
        assert_eq!(owned.frame_with(operator_env), Frame::Plain);
        assert_eq!(owned.row_separation_with(operator_env), RowSeparation::Rule);
    }

    #[test]
    fn style_env_names_can_be_replaced_or_disabled() {
        let aliased = |name: &str| (name == "TOY_RECORD").then(|| "keys-left".to_owned());
        let replaced = RenderOptions::new(ColorMode::Never)
            .record_style_envs(&["TOY_RECORD", "CTL_CORE_RECORD_STYLE"]);
        assert_eq!(replaced.record_style_with(aliased), RecordStyle::KeysLeft);

        let disabled = RenderOptions::new(ColorMode::Never)
            .record_style_envs(&[])
            .frame_envs(&[])
            .row_separation_envs(&[]);
        assert_eq!(
            disabled.record_style_with(operator_env),
            RecordStyle::KeysRight
        );
        assert_eq!(disabled.frame_with(operator_env), Frame::Grid);
        assert_eq!(
            disabled.row_separation_with(operator_env),
            RowSeparation::None
        );
    }

    #[cfg(feature = "app")]
    #[test]
    fn an_unknown_style_value_is_ignored_and_the_warning_names_the_choices() {
        let typo = |name: &str| (name == "CTL_CORE_RECORD_STYLE").then(|| "boxy".to_owned());
        let options = RenderOptions::new(ColorMode::Never);
        assert_eq!(options.record_style_with(typo), RecordStyle::KeysRight);
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
    fn a_notice_separator_takes_the_plain_style() {
        let underline = Style::new().effects(Effects::UNDERLINE);
        let theme = Theme::new().style(Role::Plain, underline);
        let rendered = Document::new()
            .notice(Notice::new(NoticeLevel::Warning, "one stale row"))
            .render(RenderOptions::new(ColorMode::Always).theme(theme).width(60));
        assert!(rendered.contains("\u{1b}[4m · "), "{rendered}");
    }

    #[test]
    fn ids_are_bold_without_a_colour() {
        let table = Table::new(["id", "title"])
            .id_column(0)
            .row(["task-014", "Title"]);
        let rendered = Document::new()
            .table(table)
            .paragraph(Text::new().id("task-015"))
            .render(RenderOptions::new(ColorMode::Always).width(60));
        assert!(
            rendered.contains("\u{1b}[1mtask-014\u{1b}[0m"),
            "{rendered}"
        );
        assert!(
            rendered.contains("\u{1b}[1mtask-015\u{1b}[0m"),
            "{rendered}"
        );
    }

    #[test]
    fn headings_wrap_to_the_width() {
        let rendered = Document::new()
            .heading("task-001  a title that runs past the line")
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

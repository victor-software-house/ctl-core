//! Pretty, colorless, and JSON emission from one typed model.

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use serde::Serialize;

use crate::color::ColorMode;
use crate::document::{Document, Text};
use crate::format::OutputFormat;
use crate::model::{Envelope, ErrorBody};
use crate::render::{EnvChoice, RenderOptions, env_choice, process_env};
use crate::theme::Theme;

/// Default environment variable for the operator's JSON layout.
pub const DEFAULT_JSON_LAYOUT_ENV: &str = "CTL_CORE_JSON_LAYOUT";
/// Default ordered environment lookup for the JSON layout.
pub const DEFAULT_JSON_LAYOUT_ENVS: &[&str] = &[DEFAULT_JSON_LAYOUT_ENV];

/// A serializable domain model with one semantic human presentation.
pub trait Present: Serialize {
    /// Build the human document. JSON serializes `self` directly.
    fn present(&self) -> Document;

    /// Whether this value represents success or failure.
    fn message_kind(&self) -> MessageKind {
        MessageKind::Success
    }

    /// Process exit code. Typed protocols can distinguish usage errors from
    /// operational failures without changing their presentation stream.
    fn exit_code(&self) -> u8 {
        self.message_kind().default_exit_code()
    }
}

/// Human stream and exit semantics for a presented model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageKind {
    /// Successful command result.
    #[default]
    Success,
    /// Failed command result.
    Error,
}

impl MessageKind {
    /// Default process exit code.
    #[must_use]
    pub const fn default_exit_code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Error => 1,
        }
    }
}

/// Where a view sends rendered output.
///
/// It is not [`ctl_term::Stream`](crate::term::Stream): that one names a
/// stream colour and width are decided for, and this one also names no
/// output at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Destination {
    /// No output, used for quiet successful pretty output.
    None,
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Rendered bytes plus their destination and exit semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Captured {
    stream: Destination,
    content: String,
    exit_code: u8,
}

impl Captured {
    /// Destination stream.
    #[must_use]
    pub const fn stream(&self) -> Destination {
        self.stream
    }

    /// Rendered bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }

    /// UTF-8 rendered content.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.content
    }

    /// Process exit code.
    #[must_use = "return it from main, or the command exits 0"]
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit_code)
    }
}

/// Line layout of JSON output. No layout adds ANSI.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum JsonLayout {
    /// One line per document.
    Compact,
    /// Two-space indentation.
    #[default]
    Pretty,
    /// Two-space indentation when stdout is a terminal, one line otherwise.
    PrettyOnTerminal,
}

impl EnvChoice for JsonLayout {
    const VALUES: &'static [(&'static str, Self)] = &[
        ("pretty", Self::Pretty),
        ("compact", Self::Compact),
        ("pretty-on-terminal", Self::PrettyOnTerminal),
    ];
}

impl JsonLayout {
    fn pretty(self) -> bool {
        match self {
            Self::Compact => false,
            Self::Pretty => true,
            Self::PrettyOnTerminal => io::stdout().is_terminal(),
        }
    }
}

/// How to present a model. JSON never contains ANSI.
///
/// Pretty output decides colour and width for the stream it goes to: a
/// success for stdout, a failure for stderr. So a failure printed to a
/// terminal is coloured while a success piped to a file is not, and
/// [`Self::capture`] under [`ColorMode::Auto`] holds no escape sequence when
/// that stream is not a terminal.
///
/// The colour, width, and style settings are one [`RenderOptions`], which
/// [`Self::render_options`] returns. A `with_` method changes a setting and
/// the method without the prefix reads it, as on `RenderOptions`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct View {
    /// Pretty or JSON.
    pub format: OutputFormat,
    /// Suppress successful pretty output.
    pub quiet: bool,
    options: RenderOptions,
    json_layout: Option<JsonLayout>,
    json_layout_envs: &'static [&'static str],
}

impl View {
    /// Build a view that prints successes.
    #[must_use]
    pub const fn new(format: OutputFormat, color: ColorMode) -> Self {
        Self {
            format,
            quiet: false,
            options: RenderOptions::new(color),
            json_layout: None,
            json_layout_envs: DEFAULT_JSON_LAYOUT_ENVS,
        }
    }

    /// Lay out JSON output as `layout`. This beats the environment.
    #[must_use]
    pub const fn with_json_layout(mut self, layout: JsonLayout) -> Self {
        self.json_layout = Some(layout);
        self
    }

    /// Replace the ordered environment names read for the JSON layout.
    /// An empty slice disables lookup.
    #[must_use]
    pub const fn with_json_layout_envs(mut self, names: &'static [&'static str]) -> Self {
        self.json_layout_envs = names;
        self
    }

    /// JSON layout: the owner's choice, then the environment, then the
    /// default. It reads the process environment.
    #[must_use]
    pub fn resolved_json_layout(self) -> JsonLayout {
        self.json_layout_with(process_env)
    }

    pub(crate) fn json_layout_with(self, value: impl Fn(&str) -> Option<String>) -> JsonLayout {
        self.json_layout
            .or_else(|| env_choice(self.json_layout_envs, value))
            .unwrap_or_default()
    }

    /// One line per style or JSON layout variable whose value is not
    /// accepted. `App` prints them before it runs.
    #[cfg(feature = "app")]
    pub(crate) fn env_warnings(self, value: impl Fn(&str) -> Option<String>) -> Vec<String> {
        let mut warnings = self.options.style_env_warnings(&value);
        if self.json_layout.is_none() {
            warnings.extend(crate::render::env_choice_warnings::<JsonLayout>(
                self.json_layout_envs,
                &value,
            ));
        }
        warnings
    }

    /// Take the theme, record style, list frame, row separation, and the
    /// variables they read from `styles`. Its colour and width settings are
    /// ignored; the view owns those.
    #[must_use]
    pub const fn with_styles(mut self, styles: RenderOptions) -> Self {
        self.options = self.options.with_styles_of(styles);
        self
    }

    /// Draw each role in the style `theme` gives it, in place of
    /// [`Theme::default`]. Colour is still [`Self::color`]'s choice:
    /// [`ColorMode::Never`] writes no style. A later [`Self::with_styles`]
    /// replaces this theme with its own.
    #[must_use]
    pub const fn with_theme(mut self, theme: Theme) -> Self {
        self.options = self.options.with_theme(theme);
        self
    }

    /// Write colour as `color` says, in place of the mode given to
    /// [`Self::new`].
    #[must_use]
    pub const fn with_color(mut self, color: ColorMode) -> Self {
        self.options = self.options.with_color(color);
        self
    }

    /// Suppress successful pretty output when `quiet` is set.
    #[must_use]
    pub const fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Force an explicit presentation width.
    #[must_use]
    pub const fn with_width(mut self, width: u16) -> Self {
        self.options = self.options.with_width(width);
        self
    }

    /// Override the buffer subtracted from automatically detected widths.
    /// Zero disables buffering. Explicit [`Self::with_width`] remains exact.
    #[must_use]
    pub const fn with_automatic_width_buffer(mut self, columns: u16) -> Self {
        self.options = self.options.with_automatic_width_buffer(columns);
        self
    }

    /// Replace the ordered environment names used for the automatic buffer.
    #[must_use]
    pub const fn with_automatic_width_buffer_envs(
        mut self,
        names: &'static [&'static str],
    ) -> Self {
        self.options = self.options.with_automatic_width_buffer_envs(names);
        self
    }

    /// Set the floor for automatically detected effective widths.
    #[must_use]
    pub const fn with_minimum_automatic_width(mut self, columns: u16) -> Self {
        self.options = self.options.with_minimum_automatic_width(columns);
        self
    }

    /// Lay out to `width` when no width is detected; `None` disables it.
    #[must_use]
    pub const fn with_fallback_width(mut self, width: Option<u16>) -> Self {
        self.options = self.options.with_fallback_width(width);
        self
    }

    /// ANSI policy for pretty output.
    #[must_use]
    pub const fn color(self) -> ColorMode {
        self.options.color()
    }

    /// The colour, width, and style settings pretty output renders with:
    /// the buffer, its environment names, the minimum, the fallback, and the
    /// styles.
    #[must_use]
    pub const fn render_options(self) -> RenderOptions {
        self.options
    }

    /// One JSON document in this view's layout, newline-terminated. Successes
    /// and error envelopes share it, so both follow the same layout.
    fn json(self, value: &impl Serialize) -> io::Result<String> {
        let mut content = if self.resolved_json_layout().pretty() {
            serde_json::to_string_pretty(value)?
        } else {
            serde_json::to_string(value)?
        };
        content.push('\n');
        Ok(content)
    }

    /// `document` in the colour and at the width decided for `stream`.
    fn pretty(self, stream: ctl_term::Stream, document: &Document) -> String {
        self.options.render_for(stream, document)
    }

    /// Render without writing. Tests and alternate transports use this path.
    ///
    /// Pretty output is rendered as [`Self::show`] would write it: colour and
    /// width are decided for the stream [`Captured::stream`] names, from the
    /// colour mode, the process environment, and whether that stream is a
    /// terminal.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` does not serialize to JSON.
    pub fn capture(self, value: &impl Present) -> io::Result<Captured> {
        let kind = value.message_kind();
        let exit_code = value.exit_code();
        if self.format.is_json() {
            return Ok(Captured {
                stream: Destination::Stdout,
                content: self.json(value)?,
                exit_code,
            });
        }
        if self.quiet && kind == MessageKind::Success {
            return Ok(Captured {
                stream: Destination::None,
                content: String::new(),
                exit_code,
            });
        }
        let (stream, target) = match kind {
            MessageKind::Success => (Destination::Stdout, ctl_term::Stream::Stdout),
            MessageKind::Error => (Destination::Stderr, ctl_term::Stream::Stderr),
        };
        Ok(Captured {
            stream,
            content: self.pretty(target, &value.present()),
            exit_code,
        })
    }

    /// Render and write one typed model. Returns its process exit semantics.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` does not serialize to JSON, or when
    /// writing to stdout or stderr fails.
    pub fn show(self, value: &impl Present) -> io::Result<ExitCode> {
        let captured = self.capture(value)?;
        match captured.stream() {
            Destination::None => {}
            Destination::Stdout => write_stdout(captured.bytes())?,
            Destination::Stderr => write_stderr(captured.bytes())?,
        }
        Ok(captured.exit_code())
    }

    /// Emit `{bin}: {message}` or a JSON error envelope.
    ///
    /// # Errors
    ///
    /// Returns an error when writing to stdout or stderr fails.
    pub fn emit_err(self, bin: &str, message: &str) -> io::Result<ExitCode> {
        if self.format.is_json() {
            let envelope = Envelope::<()>::err(ErrorBody::new(bin, message));
            write_stdout(self.json(&envelope)?.as_bytes())?;
            return Ok(ExitCode::FAILURE);
        }
        let document =
            Document::new().paragraph(Text::new().error(bin).then(": ").then(message.to_owned()));
        write_stderr(self.pretty(ctl_term::Stream::Stderr, &document).as_bytes())?;
        Ok(ExitCode::FAILURE)
    }
}

/// Write bytes to stdout as they are. Pretty output already holds exactly
/// the escapes decided for its stream, and JSON and protocol text hold none.
pub(crate) fn write_stdout(bytes: &[u8]) -> io::Result<()> {
    let mut stream = io::stdout().lock();
    stream.write_all(bytes)?;
    stream.flush()
}

/// Write bytes to stderr as they are; see [`write_stdout`].
pub(crate) fn write_stderr(bytes: &[u8]) -> io::Result<()> {
    let mut stream = io::stderr().lock();
    stream.write_all(bytes)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::View;
    use crate::color::ColorMode;
    use crate::format::OutputFormat;
    use crate::model::{Envelope, ErrorBody};
    use crate::view::JsonLayout;

    #[test]
    fn error_envelopes_follow_the_json_layout() {
        let envelope = Envelope::<()>::err(ErrorBody::new("toy", "failed"));
        let view = View::new(OutputFormat::Json, ColorMode::Never);
        assert!(view.json(&envelope).unwrap().contains("\n  "));
        let compact = view
            .with_json_layout(JsonLayout::Compact)
            .json(&envelope)
            .unwrap();
        assert_eq!(compact.lines().count(), 1, "{compact}");
    }

    #[test]
    fn the_environment_picks_the_json_layout_the_owner_left_open() {
        let compact = |name: &str| (name == "CTL_CORE_JSON_LAYOUT").then(|| "compact".to_owned());
        let view = View::new(OutputFormat::Json, ColorMode::Never);
        assert_eq!(view.json_layout_with(compact), JsonLayout::Compact);
        assert_eq!(
            view.with_json_layout(JsonLayout::Pretty)
                .json_layout_with(compact),
            JsonLayout::Pretty
        );
        assert_eq!(
            view.with_json_layout_envs(&[]).json_layout_with(compact),
            JsonLayout::Pretty
        );
    }

    #[cfg(feature = "app")]
    #[test]
    fn view_warnings_cover_styles_and_the_json_layout() {
        let typos = |name: &str| match name {
            "CTL_CORE_LIST_STYLE" => Some("boxes".to_owned()),
            "CTL_CORE_JSON_LAYOUT" => Some("tight".to_owned()),
            _ => None,
        };
        let view = View::new(OutputFormat::Pretty, ColorMode::Never);
        assert_eq!(
            view.env_warnings(typos),
            [
                "CTL_CORE_LIST_STYLE=boxes is ignored; use one of grid, header-rule, plain",
                "CTL_CORE_JSON_LAYOUT=tight is ignored; use one of pretty, compact, pretty-on-terminal",
            ]
        );
    }
}

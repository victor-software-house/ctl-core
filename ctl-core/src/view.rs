//! Pretty, colorless, and JSON output from one typed model.
//!
//! A command result implements [`Present`]: it serializes for JSON and builds
//! a [`Document`] for people. A [`View`] holds the output format, the colour
//! mode, and the layout policy, and writes the result with [`View::show`] or
//! returns it unwritten with [`View::capture`].
//!
//! Success goes to stdout and failure to stderr, except JSON, which always
//! goes to stdout. JSON never contains ANSI whatever the colour mode says.
//!
//! ```
//! use ctl_core::document::{Document, Fields};
//! use ctl_core::view::{JsonLayout, Present, Stream, View};
//! use ctl_core::{ColorMode, OutputFormat};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Status {
//!     pending: usize,
//! }
//!
//! impl Present for Status {
//!     fn present(&self) -> Document {
//!         Document::new().fields(Fields::new().row("pending", self.pending.to_string()))
//!     }
//! }
//!
//! let status = Status { pending: 2 };
//! let json = View::new(OutputFormat::Json, ColorMode::Never)
//!     .json_layout(JsonLayout::Pretty)
//!     .capture(&status)?;
//! assert_eq!(json.stream(), Stream::Stdout);
//! assert!(json.text().contains("\"pending\": 2"));
//!
//! let pretty = View::new(OutputFormat::Pretty, ColorMode::Never)
//!     .width(40)
//!     .capture(&status)?;
//! assert!(pretty.text().contains("pending"));
//! # Ok::<(), std::io::Error>(())
//! ```

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use serde::Serialize;

use crate::color::ColorMode;
use crate::document::{Document, Text};
use crate::format::OutputFormat;
use crate::model::{Envelope, ErrorBody};
use crate::render::{EnvChoice, RenderOptions, env_choice, process_env};
use crate::theme::Theme;

/// The environment variable that sets the JSON layout when the library owner
/// has not: `CTL_CORE_JSON_LAYOUT`. Its values are the [`JsonLayout`] names
/// `pretty`, `compact`, and `pretty-on-terminal`.
pub const DEFAULT_JSON_LAYOUT_ENV: &str = "CTL_CORE_JSON_LAYOUT";
/// The ordered list of variables [`View::layout`] reads, holding only
/// [`DEFAULT_JSON_LAYOUT_ENV`]. [`View::json_layout_envs`] replaces it.
pub const DEFAULT_JSON_LAYOUT_ENVS: &[&str] = &[DEFAULT_JSON_LAYOUT_ENV];

/// A command result that serializes for machines and presents itself to people.
///
/// JSON output is `self` serialized as is. Pretty and colorless output is the
/// [`Document`] that [`Self::present`] builds. The two share one value, so they
/// cannot drift apart.
pub trait Present: Serialize {
    /// Build the human document. JSON output serializes `self` and ignores
    /// this.
    fn present(&self) -> Document;

    /// Whether the result is a success or a failure. It picks the stream of
    /// pretty output (stdout or stderr) and the default exit code. The default
    /// is [`MessageKind::Success`].
    fn message_kind(&self) -> MessageKind {
        MessageKind::Success
    }

    /// The process exit code. The default is `0` for a success and `1` for an
    /// error ([`MessageKind::default_exit_code`]).
    ///
    /// Override it to give a failure its own code, such as a usage error
    /// distinct from an operational failure. The stream still follows
    /// [`Self::message_kind`].
    fn exit_code(&self) -> u8 {
        self.message_kind().default_exit_code()
    }
}

/// Whether a [`Present`] result succeeded. It picks the stream pretty output
/// goes to and the default exit code.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageKind {
    /// The command succeeded. Pretty output goes to stdout, and the exit code
    /// is `0`.
    #[default]
    Success,
    /// The command failed. Pretty output goes to stderr, and the exit code is
    /// `1`.
    Error,
}

impl MessageKind {
    /// The exit code for a result of this variant, unless the result
    /// overrides [`Present::exit_code`]: `0` for a success, `1` for an error.
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

/// A rendered result that has not been written: its text, the stream it
/// belongs on, and its exit code.
///
/// [`View::capture`] returns one. [`View::show`] writes it. Tests and
/// alternate transports read it directly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Captured {
    stream: Destination,
    content: String,
    exit_code: u8,
}

impl Captured {
    /// The stream the text belongs on.
    #[must_use]
    pub const fn stream(&self) -> Destination {
        self.stream
    }

    /// The rendered text as UTF-8 bytes. It is empty when the stream is
    /// [`Stream::None`].
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }

    /// The rendered text. It ends with a newline unless it is empty.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.content
    }

    /// The exit code to return from `main`.
    #[must_use = "return it from main, or the command exits 0"]
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit_code)
    }
}

/// How JSON output is broken into lines. No layout adds ANSI.
///
/// The layout changes only whitespace, so every layout parses to the same
/// value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum JsonLayout {
    /// The whole document on one line.
    Compact,
    /// One value per line, indented by two spaces.
    #[default]
    Pretty,
    /// [`Self::Pretty`] when stdout is a terminal, and [`Self::Compact`]
    /// otherwise.
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

/// How to present a [`Present`] result: the format, the colour mode, and the
/// layout policy. JSON never contains ANSI.
///
/// Pretty output decides colour and width for the stream it goes to: a
/// success for stdout, a failure for stderr. So a failure printed to a
/// terminal is coloured while a success piped to a file is not, and
/// [`Self::capture`] under [`ColorMode::Auto`] holds no escape sequence when
/// that stream is not a terminal.
///
/// The colour, width, and style settings are one [`RenderOptions`], which
/// [`Self::render_options`] returns. A setter takes the name of its setting
/// and returns the view changed, as on `RenderOptions`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct View {
    /// Pretty or JSON output.
    pub format: OutputFormat,
    /// Suppress successful pretty output.
    pub quiet: bool,
    options: RenderOptions,
    json_layout: Option<JsonLayout>,
    json_layout_envs: &'static [&'static str],
}

impl View {
    /// Build a view in `format` and `color` that prints successes. Width,
    /// buffer, minimum, fallback, styles, and JSON layout take their defaults.
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
    pub const fn json_layout(mut self, layout: JsonLayout) -> Self {
        self.json_layout = Some(layout);
        self
    }

    /// Replace the ordered environment names read for the JSON layout.
    /// An empty slice disables lookup.
    #[must_use]
    pub const fn json_layout_envs(mut self, names: &'static [&'static str]) -> Self {
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
    pub const fn styles(mut self, styles: RenderOptions) -> Self {
        self.options = self.options.styles(styles);
        self
    }

    /// Draw each role in the style `theme` gives it, in place of
    /// [`Theme::default`]. Colour is still the colour mode's choice:
    /// [`ColorMode::Never`] writes no style. A later [`Self::styles`]
    /// replaces this theme with its own.
    #[must_use]
    pub const fn theme(mut self, theme: Theme) -> Self {
        self.options = self.options.theme(theme);
        self
    }

    /// Write colour as `color` says, in place of the mode given to
    /// [`Self::new`].
    #[must_use]
    pub const fn color(mut self, color: ColorMode) -> Self {
        self.options = self.options.color(color);
        self
    }

    /// Suppress a successful pretty result when `quiet` is true. Errors and
    /// JSON are still written.
    #[must_use]
    pub const fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Lay out pretty output to exactly `width` terminal cells, skipping
    /// detection, the buffer, and the minimum.
    #[must_use]
    pub const fn width(mut self, width: u16) -> Self {
        self.options = self.options.width(width);
        self
    }

    /// Override the number of columns subtracted from an automatically
    /// detected width. Zero disables the subtraction. An explicit
    /// [`Self::width`] stays exact.
    #[must_use]
    pub const fn automatic_width_buffer(mut self, columns: u16) -> Self {
        self.options = self.options.automatic_width_buffer(columns);
        self
    }

    /// Replace the ordered environment names that set the buffer. An empty
    /// slice disables lookup.
    #[must_use]
    pub const fn automatic_width_buffer_envs(mut self, names: &'static [&'static str]) -> Self {
        self.options = self.options.automatic_width_buffer_envs(names);
        self
    }

    /// Set the floor, in columns, for an automatically detected width.
    #[must_use]
    pub const fn minimum_automatic_width(mut self, columns: u16) -> Self {
        self.options = self.options.minimum_automatic_width(columns);
        self
    }

    /// Lay out to `width` columns when no width is detected. `None` turns the
    /// fallback off, so tables render at their natural width.
    #[must_use]
    pub const fn fallback_width(mut self, width: Option<u16>) -> Self {
        self.options = self.options.fallback_width(width);
        self
    }

    /// The colour, width, and style settings pretty output renders with:
    /// the buffer, its environment names, the minimum, the fallback, and the
    /// styles.
    #[must_use]
    pub const fn render_options(self) -> RenderOptions {
        self.options
    }

    /// One JSON document in this view's layout, ending in a newline. A success
    /// and an error envelope share it, so both follow the same layout.
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

    /// Render `value` without writing it. Tests and alternate transports use
    /// this path.
    ///
    /// JSON output is `value` serialized in the [`Self::layout`], for stdout.
    /// Pretty output is rendered as [`Self::show`] would write it. Colour and
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

    /// Render `value` and write it to the stream [`Self::capture`] picks.
    /// Returns the exit code for `main` to return.
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

    /// Write a failure that has no [`Present`] model, such as a parse error.
    ///
    /// Pretty output is `{bin}: {message}` on stderr. JSON output is an
    /// [`Envelope`] with an [`ErrorBody`] on stdout, in this view's layout.
    /// The exit code is [`ExitCode::FAILURE`] in both.
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
            .json_layout(JsonLayout::Compact)
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
            view.json_layout(JsonLayout::Pretty)
                .json_layout_with(compact),
            JsonLayout::Pretty
        );
        assert_eq!(
            view.json_layout_envs(&[]).json_layout_with(compact),
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

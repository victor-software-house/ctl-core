//! Pretty, colourless, and JSON output from one typed model.
//!
//! A command result implements [`Present`]: it serializes for JSON and builds
//! a [`Document`] for people. A [`View`] holds the output [`Format`], the
//! colour mode, and the layout policy, and writes the result with
//! [`View::show`] or returns it unwritten with [`View::capture`].
//!
//! Success goes to stdout and failure to stderr, in JSON as in pretty output.
//! A JSON failure leaves stdout empty, so a reader that parses stdout never
//! mistakes an error for a result. A machine format never contains ANSI
//! whatever the colour mode says.
//!
//! ```
//! use ctl_core::color::ColorMode;
//! use ctl_core::document::{Document, Fields};
//! use ctl_core::format::{JSON, PRETTY};
//! use ctl_core::view::{Destination, JsonLayout, Present, View};
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
//! let json = View::new(&JSON, ColorMode::Never)
//!     .json_layout(JsonLayout::Pretty)
//!     .capture(&status)?;
//! assert_eq!(json.stream(), Destination::Stdout);
//! assert!(json.text().contains("\"pending\": 2"));
//!
//! let pretty = View::new(&PRETTY, ColorMode::Never)
//!     .width(40)
//!     .capture(&status)?;
//! assert!(pretty.text().contains("pending"));
//! # Ok::<(), std::io::Error>(())
//! ```

use std::fmt;
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use serde::Serialize;

use crate::color::ColorMode;
use crate::document::Document;
use crate::format::{
    CSV, Format, FormatName, Formats, JSON, MARKDOWN, Output, PRETTY, Sink, TSV, unregistered,
};
use crate::model::ErrorBody;
#[cfg(feature = "app")]
use crate::render;
use crate::render::{EnvChoice, RenderOptions, env_choice, process_env};
use crate::theme::Theme;

mod records;

pub use records::{Records, Reply, Respond};

/// The environment variable that sets the JSON layout when the CLI has not:
/// `CTL_CORE_JSON_LAYOUT`. Its values are the [`JsonLayout`] names
/// `pretty`, `compact`, and `pretty-on-terminal`.
pub const DEFAULT_JSON_LAYOUT_ENV: &str = "CTL_CORE_JSON_LAYOUT";
/// The ordered list of variables [`View::resolved_json_layout`] reads,
/// holding only [`DEFAULT_JSON_LAYOUT_ENV`]. [`View::json_layout_envs`]
/// replaces it.
pub const DEFAULT_JSON_LAYOUT_ENVS: &[&str] = &[DEFAULT_JSON_LAYOUT_ENV];

/// A command result that serializes for machines and presents itself to people.
///
/// JSON output is `self` serialized as is. Pretty and colourless output is the
/// [`Document`] that [`Self::present`] builds. The two share one value, so they
/// cannot drift apart.
pub trait Present: Serialize {
    /// Build the human document. JSON output serializes `self` and ignores
    /// this.
    fn present(&self) -> Document;

    /// Whether the result is a success or a failure. It picks the stream
    /// (stdout or stderr) and the default exit code. The default is
    /// [`Outcome::Success`].
    fn outcome(&self) -> Outcome {
        Outcome::Success
    }

    /// The process exit code. The default is `0` for a success and `1` for an
    /// error ([`Outcome::default_exit_code`]).
    ///
    /// Override it to give a failure its own code, such as a usage error
    /// distinct from an operational failure. The stream still follows
    /// [`Self::outcome`].
    fn exit_code(&self) -> u8 {
        self.outcome().default_exit_code()
    }
}

/// Whether a [`Present`] result succeeded. It picks the stream the output
/// goes to and the default exit code.
///
/// The set is closed, and the enum is not `#[non_exhaustive]`: a command
/// either succeeded or failed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Outcome {
    /// The command succeeded. Its output goes to stdout, and the exit code
    /// is `0`.
    #[default]
    Success,
    /// The command failed. Its output, pretty or JSON, goes to stderr, and the
    /// exit code is `1`.
    Error,
}

impl Outcome {
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
/// [`ctl_term::Stream`](crate::term::Stream) names the stream that colour
/// and width are decided for. A destination can also be no output at all.
///
/// The set is closed, and the enum is not `#[non_exhaustive]`: a process has
/// one standard output and one standard error, and quiet output goes to
/// neither.
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
    /// [`Destination::None`].
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
    /// [`Self::Pretty`] or [`Self::Compact`]: `PrettyOnTerminal` settled by
    /// whether stdout is a terminal, as it has always been, for a failure on
    /// stderr too.
    pub(crate) fn settled(self) -> Self {
        match self {
            Self::Pretty => Self::Pretty,
            Self::PrettyOnTerminal if io::stdout().is_terminal() => Self::Pretty,
            Self::Compact | Self::PrettyOnTerminal => Self::Compact,
        }
    }
}

/// How to present a [`Present`] result: the format, the colour mode, and the
/// layout policy. A machine format never contains ANSI.
///
/// Pretty output decides colour and width for the stream it goes to: a
/// success for stdout, a failure for stderr. So a failure printed to a
/// terminal is coloured while a success piped to a file is not.
/// [`Self::capture`] under [`ColorMode::Auto`] holds no escape sequence when
/// that stream is not a terminal.
///
/// The colour, width, and style settings are one [`RenderOptions`], which
/// [`Self::render_options`] returns. A setter takes the name of its setting
/// and returns the view changed, as on `RenderOptions`. [`Self::format`] and
/// [`Self::is_quiet`] read the format and the quiet setting.
///
/// A view holds one of four choices of format, and only the first two are
/// ready to write. [`Self::new`] names a registered format. A view built by
/// [`Self::requested`] holds a name, or none, until [`Self::resolve`] looks it
/// up in a registry and picks a default: the terminal or piped default, or the
/// stream default under [`Self::stream`]. [`Self::capture`]
/// resolves such a view against the built-in formats itself. `--quiet` hides a
/// successful result of a format that is not a machine format, and of a
/// default; it never hides a machine format that was named.
#[derive(Clone, Copy)]
pub struct View {
    choice: Choice,
    quiet: bool,
    options: RenderOptions,
    json_layout: Option<JsonLayout>,
    json_layout_envs: &'static [&'static str],
    script: bool,
    stream: bool,
}

/// The formats ctl-core itself offers, which a view that was never resolved
/// against a registry is looked up in.
static BUILTIN: [&dyn Format; 5] = [&PRETTY, &JSON, &TSV, &CSV, &MARKDOWN];

/// How a view knows its format. A named format was chosen by a flag, a
/// variable, a shorthand, or the caller. A defaulted one came from the
/// registry's default, and `--quiet` hides it even when it is a machine
/// format.
#[derive(Clone, Copy)]
enum Choice {
    Named(&'static dyn Format),
    Defaulted(&'static dyn Format),
    Requested(FormatName),
    Unset,
}

impl Choice {
    /// The name this choice stands for, when it holds one.
    fn name(self) -> Option<FormatName> {
        match self {
            Self::Named(format) | Self::Defaulted(format) => Some(format.name()),
            Self::Requested(name) => Some(name),
            Self::Unset => None,
        }
    }

    /// The name of this kind of choice, for `Debug`.
    const fn kind(self) -> &'static str {
        match self {
            Self::Named(_) => "named",
            Self::Defaulted(_) => "defaulted",
            Self::Requested(_) => "requested",
            Self::Unset => "unset",
        }
    }

    /// Which of the four this is, with a registered format compared by name.
    fn key(self) -> (u8, Option<FormatName>) {
        match self {
            Self::Named(_) => (0, self.name()),
            Self::Defaulted(_) => (1, self.name()),
            Self::Requested(_) => (2, self.name()),
            Self::Unset => (3, None),
        }
    }
}

impl View {
    /// Build a view that writes in the registered `format`, in `color`, and
    /// prints successes. Width, buffer, minimum, fallback, styles, and JSON
    /// layout take their defaults.
    #[must_use]
    pub const fn new(format: &'static dyn Format, color: ColorMode) -> Self {
        Self::with_choice(Choice::Named(format), color)
    }

    /// Build a view for the format `name` that a flag or a variable named, or
    /// for none: [`Self::resolve`] then picks the default. Until it resolves,
    /// [`Self::capture`] looks the name up among the built-in formats.
    #[must_use]
    pub const fn requested(name: Option<FormatName>, color: ColorMode) -> Self {
        let choice = match name {
            Some(name) => Choice::Requested(name),
            None => Choice::Unset,
        };
        Self::with_choice(choice, color)
    }

    const fn with_choice(choice: Choice, color: ColorMode) -> Self {
        Self {
            choice,
            quiet: false,
            options: RenderOptions::new(color),
            json_layout: None,
            json_layout_envs: DEFAULT_JSON_LAYOUT_ENVS,
            script: false,
            stream: false,
        }
    }

    /// The view for a script a shell verb prints: any format but `json` writes
    /// the script as plain text, and quiet never hides it.
    #[cfg(feature = "shell")]
    #[must_use]
    pub(crate) const fn script(mut self) -> Self {
        self.script = true;
        self
    }

    /// The view with the format that `registry` and the terminal check settle:
    /// a requested name is looked up, and no name takes the terminal or the
    /// piped default, or the stream default under [`Self::stream`]. A view that
    /// already names a format is returned as it is, and a script view writes
    /// every format but `json` as `pretty`.
    ///
    /// # Errors
    ///
    /// An [`io::ErrorKind::InvalidInput`] error when the requested name, or
    /// the default, is not registered.
    pub fn resolve(mut self, formats: &Formats, terminal: bool) -> io::Result<Self> {
        self.choice = match self.choice {
            Choice::Named(_) | Choice::Defaulted(_) => self.choice,
            Choice::Requested(name) if self.prints_as_pretty(name) => Choice::Named(&PRETTY),
            Choice::Requested(name) => Choice::Named(
                formats
                    .get(name)
                    .ok_or_else(|| unregistered("the format", name))?,
            ),
            Choice::Unset if self.script => Choice::Named(&PRETTY),
            Choice::Unset if self.stream => Choice::Defaulted(formats.stream_format()?),
            Choice::Unset => Choice::Defaulted(formats.default_format(terminal)?),
        };
        Ok(self)
    }

    /// The view asking for `name` in place of the request it holds, when the
    /// caller found one and the view still holds a request. A view that names
    /// or defaults a format is returned as it is, and so is one when no name
    /// was found, so the view the caller chose stands.
    #[cfg(feature = "app")]
    pub(crate) const fn with_request(mut self, name: Option<FormatName>) -> Self {
        if let Some(name) = name
            && matches!(self.choice, Choice::Requested(_) | Choice::Unset)
        {
            self.choice = Choice::Requested(name);
        }
        self
    }

    /// This view with its format settled against the built-in formats and
    /// the terminal check, as [`Self::capture`] does. A named or defaulted view
    /// is returned at once, a requested name is looked up among the built-ins,
    /// and only a view with no request asks the terminal.
    fn settled(self) -> io::Result<Self> {
        match self.choice {
            Choice::Named(_) | Choice::Defaulted(_) => Ok(self),
            Choice::Requested(name) if self.prints_as_pretty(name) => {
                Ok(self.with_choice_of(Choice::Named(&PRETTY)))
            }
            Choice::Requested(name) => BUILTIN
                .iter()
                .copied()
                .find(|format| format.name() == name)
                .map(|format| self.with_choice_of(Choice::Named(format)))
                .ok_or_else(|| unregistered("the format", name)),
            Choice::Unset => self.resolve(&Formats::builtin(), io::stdout().is_terminal()),
        }
    }

    /// Whether a script view writes `name` as `pretty`: every format but
    /// `json`.
    fn prints_as_pretty(self, name: FormatName) -> bool {
        self.script && name != FormatName::JSON
    }

    /// This view with `choice` in place of its own.
    const fn with_choice_of(mut self, choice: Choice) -> Self {
        self.choice = choice;
        self
    }

    /// The format a settled view writes in, and whether it came from a
    /// default.
    fn chosen(self) -> io::Result<(&'static dyn Format, bool)> {
        match self.choice {
            Choice::Named(format) => Ok((format, false)),
            Choice::Defaulted(format) => Ok((format, true)),
            Choice::Requested(_) | Choice::Unset => {
                Err(io::Error::other("the view's format is not resolved"))
            }
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

    /// JSON layout: the CLI's choice, then the environment, then the
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
            warnings.extend(render::env_choice_warnings::<JsonLayout>(
                self.json_layout_envs,
                &value,
            ));
        }
        warnings
    }

    /// Take the theme, record style, table frame, row separation, and the
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

    /// Write a result as it is produced when `stream` is true: `--stream`.
    ///
    /// A view with no format named then takes the registry's stream default,
    /// [`Formats::stream_default`], in place of the terminal or piped default,
    /// when [`Self::resolve`] settles it. [`Self::show`] writes a value as a
    /// stream of one record, and [`Self::show_records`] writes each record when
    /// the command produces it. [`Self::capture`] writes nothing as records, so
    /// the setting only decides which default format it resolves an unset view
    /// to.
    #[must_use]
    pub const fn stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Suppress a successful result when `quiet` is true, unless the format is
    /// a machine format that was named. Errors are still written.
    #[must_use]
    pub const fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// The name of the format in force. A view that has not resolved yet
    /// resolves against the built-in formats and the terminal check, as
    /// [`Self::capture`] does, and a name no built-in answers to is returned as
    /// it was requested.
    #[must_use]
    pub fn format(self) -> FormatName {
        let choice = self.settled().map_or(self.choice, |view| view.choice);
        choice.name().unwrap_or(FormatName::PRETTY)
    }

    /// Whether the format in force is a [machine format](Format::machine).
    /// A name that no built-in answers to is not.
    #[must_use]
    pub fn is_machine(self) -> bool {
        self.settled()
            .and_then(Self::chosen)
            .is_ok_and(|(format, _)| format.machine())
    }

    /// Whether a successful result is suppressed, as [`Self::quiet`] set it.
    #[must_use]
    pub const fn is_quiet(self) -> bool {
        self.quiet
    }

    /// Whether results are written as they are produced, as [`Self::stream`]
    /// set it.
    #[must_use]
    pub const fn is_stream(self) -> bool {
        self.stream
    }

    /// Whether the view still has no format: no flag, variable, shorthand, or
    /// caller named one.
    #[cfg(feature = "app")]
    pub(crate) const fn names_no_format(self) -> bool {
        matches!(self.choice, Choice::Unset)
    }

    /// Whether this view writes nothing for a successful result of a
    /// `format` that was `defaulted` or named: `--quiet` hides `pretty` and
    /// a default, and never a machine format that was named.
    fn hides(self, format: &dyn Format, defaulted: bool) -> bool {
        self.quiet && !self.script && (defaulted || !format.machine())
    }

    /// Lay out pretty output to exactly `width` terminal cells, skipping
    /// detection, the buffer, and the minimum.
    #[must_use]
    pub const fn width(mut self, width: u16) -> Self {
        self.options = self.options.width(width);
        self
    }

    /// Override the number of cells subtracted from an automatically
    /// detected width. Zero disables the subtraction. An explicit
    /// [`Self::width`] stays exact.
    #[must_use]
    pub const fn automatic_width_buffer(mut self, cells: u16) -> Self {
        self.options = self.options.automatic_width_buffer(cells);
        self
    }

    /// Replace the ordered environment names that set the buffer. An empty
    /// slice disables lookup.
    #[must_use]
    pub const fn automatic_width_buffer_envs(mut self, names: &'static [&'static str]) -> Self {
        self.options = self.options.automatic_width_buffer_envs(names);
        self
    }

    /// Set the floor, in cells, for an automatically detected width.
    #[must_use]
    pub const fn minimum_automatic_width(mut self, cells: u16) -> Self {
        self.options = self.options.minimum_automatic_width(cells);
        self
    }

    /// Lay out to `width` cells when no width is detected. `None` turns the
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

    /// A sink over `writer` for `stream`, with this view's options and JSON
    /// layout, and no colour when `format` is a machine format.
    fn sink<'a>(
        self,
        writer: &'a mut dyn Write,
        stream: ctl_term::Stream,
        format: &dyn Format,
    ) -> Sink<'a> {
        Sink::new(
            writer,
            stream,
            self.options,
            self.resolved_json_layout().settled(),
            format.machine(),
        )
    }

    /// Render `value` without writing it. Tests and alternate transports use
    /// this path.
    ///
    /// The format writes `value` for stdout on a success and stderr on an
    /// error, in the layout [`Self::resolved_json_layout`] returns when the
    /// format is JSON. Colour and width are decided for the stream
    /// [`Captured::stream`] names, from the colour mode, the process
    /// environment, and whether that stream is a terminal. A view that has not
    /// resolved yet resolves against the built-in formats and the terminal
    /// check.
    ///
    /// # Errors
    ///
    /// Returns an error when the view's format is not a built-in, when the
    /// format fails to write `value`, or when what it wrote is not UTF-8.
    pub fn capture(self, value: &impl Present) -> io::Result<Captured> {
        let present = || value.present();
        let output = Output::new(value, &present, value.outcome());
        self.capture_output(&output, value.exit_code())
    }

    /// [`Self::capture`] for a result already behind [`Output`], so one copy
    /// of this body serves every result type.
    fn capture_output(self, output: &Output<'_>, exit_code: u8) -> io::Result<Captured> {
        let view = self.settled()?;
        let (format, defaulted) = view.chosen()?;
        let outcome = output.outcome();
        if outcome == Outcome::Success && view.hides(format, defaulted) {
            return Ok(Captured {
                stream: Destination::None,
                content: String::new(),
                exit_code,
            });
        }
        let (stream, target) = match outcome {
            Outcome::Success => (Destination::Stdout, ctl_term::Stream::Stdout),
            Outcome::Error => (Destination::Stderr, ctl_term::Stream::Stderr),
        };
        let mut written = Vec::new();
        format.write(output, &mut view.sink(&mut written, target, format))?;
        let content = String::from_utf8(written)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(Captured {
            stream,
            content,
            exit_code,
        })
    }

    /// Render `value` and write it to the stream [`Self::capture`] picks.
    /// Returns the exit code for `main` to return.
    ///
    /// Under [`Self::stream`] a successful value is written as a stream of
    /// one record, by the format's
    /// [`RecordWriter`](crate::format::RecordWriter), with no `type` field
    /// added. A value that failed keeps its stream, stderr, and its exit code.
    ///
    /// # Errors
    ///
    /// Returns an error when the format fails to write `value`, or when
    /// writing to stdout or stderr fails.
    pub fn show(self, value: &impl Present) -> io::Result<ExitCode> {
        self.show_record(value)?;
        Ok(ExitCode::from(value.exit_code()))
    }

    /// [`Self::show`] for a result already behind [`records::Record`], so one
    /// copy of this body serves every result type.
    fn show_record(self, value: &dyn records::Record) -> io::Result<()> {
        if self.stream && value.outcome() == Outcome::Success {
            return self.stream_value(value);
        }
        let present = || value.present();
        let output = Output::new(value.data(), &present, value.outcome());
        let captured = self.capture_output(&output, value.exit_code())?;
        match captured.stream() {
            Destination::None => Ok(()),
            Destination::Stdout => write_stdout(captured.bytes()),
            Destination::Stderr => write_stderr(captured.bytes()),
        }
    }

    /// Write a failure that has no [`Present`] model, such as a parse error.
    ///
    /// The format writes its own error form to stderr, and stdout stays empty.
    /// Pretty output is `{bin}: {message}`. JSON output is an
    /// [`Envelope`](crate::model::Envelope) with an [`ErrorBody`], in this
    /// view's layout. The exit code is [`ExitCode::FAILURE`] in both.
    ///
    /// # Errors
    ///
    /// Returns an error when writing to stderr fails.
    pub fn emit_err(self, bin: &str, message: &str) -> io::Result<ExitCode> {
        self.write_error(&ErrorBody::new(bin, message))?;
        Ok(ExitCode::FAILURE)
    }

    /// Write `error` to stderr in the format's error form.
    pub(crate) fn write_error(self, error: &ErrorBody) -> io::Result<()> {
        let view = self.settled()?;
        let (format, _) = view.chosen()?;
        let mut written = Vec::new();
        format.write_error(
            error,
            &mut view.sink(&mut written, ctl_term::Stream::Stderr, format),
        )?;
        write_stderr(&written)
    }
}

impl PartialEq for View {
    fn eq(&self, other: &Self) -> bool {
        self.choice.key() == other.choice.key()
            && self.quiet == other.quiet
            && self.options == other.options
            && self.json_layout == other.json_layout
            && self.json_layout_envs == other.json_layout_envs
            && self.script == other.script
            && self.stream == other.stream
    }
}

impl Eq for View {}

/// Names the format and the kind of choice that holds it: a registered format
/// has no `Debug`.
impl fmt::Debug for View {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("View")
            .field("format", &self.choice.name())
            .field("choice", &self.choice.kind())
            .field("quiet", &self.quiet)
            .field("options", &self.options)
            .field("json_layout", &self.json_layout)
            .field("script", &self.script)
            .field("stream", &self.stream)
            .finish_non_exhaustive()
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
    use crate::format::{FormatName, JSON, PRETTY};
    use crate::view::JsonLayout;

    #[test]
    fn the_environment_picks_the_json_layout_the_owner_left_open() {
        let compact = |name: &str| (name == "CTL_CORE_JSON_LAYOUT").then(|| "compact".to_owned());
        let view = View::new(&JSON, ColorMode::Never);
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
        let view = View::new(&PRETTY, ColorMode::Never);
        assert_eq!(
            view.env_warnings(typos),
            [
                "CTL_CORE_LIST_STYLE=boxes is ignored; use one of grid, header-rule, plain",
                "CTL_CORE_JSON_LAYOUT=tight is ignored; use one of pretty, compact, pretty-on-terminal",
            ]
        );
    }

    #[test]
    fn views_compare_by_format_name_and_by_how_it_was_chosen() {
        let json = View::new(&JSON, ColorMode::Never);
        assert_eq!(json, View::new(&JSON, ColorMode::Never));
        assert_ne!(json, View::new(&PRETTY, ColorMode::Never));
        assert_ne!(
            json,
            View::requested(Some(FormatName::JSON), ColorMode::Never)
        );
        assert_eq!(json.format(), FormatName::JSON);
        assert!(json.is_machine());
        assert!(!View::new(&PRETTY, ColorMode::Never).is_machine());
    }
}

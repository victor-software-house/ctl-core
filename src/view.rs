//! Pretty, colorless, and JSON emission from one typed model.

use std::io::{self, Write};
use std::process::ExitCode;

use anstream::AutoStream;
use serde::Serialize;

use crate::color::ColorMode;
use crate::document::{Document, Text};
use crate::format::OutputFormat;
use crate::model::{Envelope, ErrorBody};
use crate::render::RenderOptions;

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

/// Destination selected by a view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stream {
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
    stream: Stream,
    content: String,
    exit_code: u8,
}

impl Captured {
    /// Destination stream.
    #[must_use]
    pub const fn stream(&self) -> Stream {
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
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit_code)
    }
}

/// How to present a model. JSON never contains ANSI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct View {
    /// Pretty or JSON.
    pub format: OutputFormat,
    /// ANSI policy for pretty output.
    pub color: ColorMode,
    /// Suppress successful pretty output.
    pub quiet: bool,
    width: Option<u16>,
}

impl View {
    /// Build a view that prints successes.
    #[must_use]
    pub const fn new(format: OutputFormat, color: ColorMode) -> Self {
        Self {
            format,
            color,
            quiet: false,
            width: None,
        }
    }

    /// Suppress successful pretty output when `quiet` is set.
    #[must_use]
    pub const fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Force an explicit presentation width.
    #[must_use]
    pub const fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Render without writing. Tests and alternate transports use this path.
    pub fn capture(self, value: &impl Present) -> io::Result<Captured> {
        let kind = value.message_kind();
        let exit_code = value.exit_code();
        if self.format.is_json() {
            let mut content = serde_json::to_string(value)?;
            content.push('\n');
            return Ok(Captured {
                stream: Stream::Stdout,
                content,
                exit_code,
            });
        }
        if self.quiet && kind == MessageKind::Success {
            return Ok(Captured {
                stream: Stream::None,
                content: String::new(),
                exit_code,
            });
        }
        let options = self.width.map_or_else(
            || RenderOptions::new(self.color),
            |width| RenderOptions::new(self.color).width(width),
        );
        let content = value.present().render(options);
        Ok(Captured {
            stream: match kind {
                MessageKind::Success => Stream::Stdout,
                MessageKind::Error => Stream::Stderr,
            },
            content,
            exit_code,
        })
    }

    /// Render and write one typed model. Returns its process exit semantics.
    pub fn show(self, value: &impl Present) -> io::Result<ExitCode> {
        let captured = self.capture(value)?;
        match captured.stream() {
            Stream::None => {}
            Stream::Stdout => write_stdout(captured.bytes(), self.color)?,
            Stream::Stderr => write_stderr(captured.bytes(), self.color)?,
        }
        Ok(captured.exit_code())
    }

    /// Emit `{bin}: {message}` or a JSON error envelope.
    pub fn emit_err(self, bin: &str, message: &str) -> io::Result<ExitCode> {
        if self.format.is_json() {
            emit_json(&Envelope::<()>::err(ErrorBody::new(bin, message)))?;
            return Ok(ExitCode::FAILURE);
        }
        let options = self.width.map_or_else(
            || RenderOptions::new(self.color),
            |width| RenderOptions::new(self.color).width(width),
        );
        let document =
            Document::new().paragraph(Text::new().error(bin).then(": ").then(message.to_owned()));
        write_stderr(document.render(options).as_bytes(), self.color)?;
        Ok(ExitCode::FAILURE)
    }
}

/// Write `value` as one JSON line to stdout. No ANSI.
pub(crate) fn emit_json<T: Serialize>(value: &T) -> io::Result<()> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer(&mut lock, value)?;
    lock.write_all(b"\n")?;
    lock.flush()
}

/// Write raw bytes to stdout with `color`.
pub(crate) fn write_stdout(bytes: &[u8], color: ColorMode) -> io::Result<()> {
    let mut stream = AutoStream::new(io::stdout().lock(), color.choice());
    stream.write_all(bytes)?;
    stream.flush()
}

/// Write raw bytes to stderr with `color`.
pub(crate) fn write_stderr(bytes: &[u8], color: ColorMode) -> io::Result<()> {
    let mut stream = AutoStream::new(io::stderr().lock(), color.choice());
    stream.write_all(bytes)?;
    stream.flush()
}

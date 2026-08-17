//! Multi-view emit: one serializable model, pretty or JSON, color or not.

use std::io::{self, Write};

use anstream::AutoStream;
use serde::Serialize;

use crate::color::ColorMode;
use crate::format::OutputFormat;
use crate::formatdoc;
use crate::model::{Envelope, ErrorBody};

/// Pretty text for a model. Prefer [`Pretty`] + a Jinja template.
pub trait Render {
    /// Human view. Must not depend on [`View::format`].
    fn render_pretty(&self) -> String;
}

/// A serializable model whose pretty view is a Jinja template.
///
/// Prepare the data. The template owns `{% if %}`, loops, and alignment.
pub trait Pretty: Serialize {
    /// `include_str!` of a `.jinja` file next to the binary crate.
    const TEMPLATE: &'static str;
}

/// Render `value` through a Jinja template. Used by [`View::show_pretty`].
pub fn render_template(source: &str, value: &impl Serialize) -> Result<String, minijinja::Error> {
    let mut env = minijinja::Environment::new();
    env.add_template("pretty", source)?;
    env.get_template("pretty")?.render(value)
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
}

impl View {
    #[must_use]
    /// Build a view that prints successes.
    pub fn new(format: OutputFormat, color: ColorMode) -> Self {
        Self {
            format,
            color,
            quiet: false,
        }
    }

    #[must_use]
    /// Suppress successful pretty output when `quiet` is set.
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// JSON writes the model. Pretty writes [`Render::render_pretty`].
    pub fn show(self, value: &(impl Serialize + Render)) -> io::Result<()> {
        self.emit(value, &value.render_pretty())
    }

    /// JSON writes the model. Pretty renders [`Pretty::TEMPLATE`] against it.
    pub fn show_pretty<T: Pretty>(self, value: &T) -> io::Result<()> {
        self.show_template(value, T::TEMPLATE)
    }

    /// JSON writes the model. Pretty renders `template` against it.
    pub fn show_template(self, value: &impl Serialize, template: &str) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }
        if self.format.is_json() {
            return emit_json(value);
        }
        let pretty = render_template(template, value).map_err(io::Error::other)?;
        write_stdout(pretty.as_bytes(), self.color)
    }

    /// JSON writes `value`. Pretty writes `pretty` (already styled or plain).
    pub fn emit(self, value: &impl Serialize, pretty: &str) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }
        if self.format.is_json() {
            return emit_json(value);
        }
        write_stdout(pretty.as_bytes(), self.color)
    }

    /// Emit a success [`Envelope`].
    pub fn emit_ok<T: Serialize>(self, data: &T, pretty: &str) -> io::Result<()> {
        self.emit(&Envelope::ok(data), pretty)
    }

    /// Emit `{bin}: {message}` or a JSON error envelope.
    pub fn emit_err(self, bin: &str, message: &str) -> io::Result<()> {
        let error = ErrorBody::new(bin, message);
        if self.format.is_json() {
            return emit_json(&Envelope::<()>::err(error));
        }
        write_stderr(formatdoc!("{bin}: {message}\n").as_bytes(), self.color)
    }
}

/// Write `value` as one JSON line to stdout. No ANSI.
pub fn emit_json<T: Serialize>(value: &T) -> io::Result<()> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer(&mut lock, value)?;
    lock.write_all(b"\n")?;
    lock.flush()
}

/// Write raw bytes to stdout with `color`.
pub fn write_stdout(bytes: &[u8], color: ColorMode) -> io::Result<()> {
    let mut stream = AutoStream::new(io::stdout().lock(), color.choice());
    stream.write_all(bytes)?;
    stream.flush()
}

/// Write raw bytes to stderr with `color`.
pub fn write_stderr(bytes: &[u8], color: ColorMode) -> io::Result<()> {
    let mut stream = AutoStream::new(io::stderr().lock(), color.choice());
    stream.write_all(bytes)?;
    stream.flush()
}

//! The writer: renders a widget at the stream's width and writes it.

use std::error::Error as StdError;
use std::fmt;
use std::io::{self, Write};

use ctl_text::{Context, Renderable};

use crate::color::ColorMode;
use crate::encode::Encoder;
use crate::stream::Stream;
use crate::terminal::{DetectOptions, Terminal};

/// Why writing failed.
///
/// A closed pipe is its own variant because it is not a failure of the
/// program: a reader such as `head` that has read enough closes the stream,
/// and the caller usually exits quietly. This crate does not choose the exit
/// code.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The reader closed the stream before the output was written.
    BrokenPipe(io::Error),
    /// Any other write or flush failure.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrokenPipe(_) => f.write_str("the reader closed the output stream"),
            Self::Io(_) => f.write_str("could not write the output"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::BrokenPipe(source) | Self::Io(source) => Some(source),
        }
    }
}

impl From<io::Error> for Error {
    /// Sorts a closed pipe from every other failure.
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::BrokenPipe {
            Self::BrokenPipe(error)
        } else {
            Self::Io(error)
        }
    }
}

/// Writes widgets to one output stream, in the colour and at the width
/// decided for that stream.
///
/// A console renders a [`Renderable`] at the stream's width, encodes the lines
/// with [`Encoder`], and writes them. With colour off it writes the plain text
/// of the lines. A stream with no width, because the fallback is off, renders
/// at `u16::MAX` cells, so a widget takes its natural width.
///
/// On Windows, a console for a terminal turns on virtual-terminal mode, which
/// the console needs to read escapes. When that fails the console writes no
/// colour.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Console {
    stream: Stream,
    terminal: Terminal,
}

impl Console {
    /// A console for `stream` that follows the decision in `terminal`.
    #[must_use]
    pub fn new(stream: Stream, terminal: Terminal) -> Self {
        Self {
            stream,
            terminal: enable_escapes(terminal),
        }
    }

    /// A console for `stream` that decides from the process environment and
    /// the stream, with the defaults of [`DetectOptions::new`].
    #[must_use]
    pub fn from_process(stream: Stream, mode: ColorMode) -> Self {
        Self::from_process_with(stream, mode, DetectOptions::new())
    }

    /// [`from_process`](Self::from_process) with `options` in place of the
    /// defaults.
    #[must_use]
    pub fn from_process_with(stream: Stream, mode: ColorMode, options: DetectOptions) -> Self {
        Self::new(stream, Terminal::from_process_with(stream, mode, options))
    }

    /// The stream this console writes to.
    #[must_use]
    pub fn stream(self) -> Stream {
        self.stream
    }

    /// What was decided for the stream.
    #[must_use]
    pub fn terminal(self) -> Terminal {
        self.terminal
    }

    /// The text [`print`](Self::print) writes for `renderable`.
    #[must_use]
    pub fn render(self, renderable: &dyn Renderable) -> String {
        let width = self.terminal.width().unwrap_or(u16::MAX);
        let lines = renderable.render(&Context::new(width));
        Encoder::new(self.terminal.color()).encode(&lines)
    }

    /// Writes `renderable` to `writer` as [`print`](Self::print) would write
    /// it to the stream, then flushes.
    ///
    /// # Errors
    ///
    /// [`Error::BrokenPipe`] when the reader has closed `writer`, and
    /// [`Error::Io`] for any other write or flush failure.
    pub fn write_to(
        self,
        writer: &mut dyn Write,
        renderable: &dyn Renderable,
    ) -> Result<(), Error> {
        writer.write_all(self.render(renderable).as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    /// Writes `renderable` to the stream, then flushes.
    ///
    /// # Errors
    ///
    /// [`Error::BrokenPipe`] when the reader has closed the stream, and
    /// [`Error::Io`] for any other write or flush failure.
    pub fn print(self, renderable: &dyn Renderable) -> Result<(), Error> {
        match self.stream {
            Stream::Stdout => self.write_to(&mut io::stdout().lock(), renderable),
            Stream::Stderr => self.write_to(&mut io::stderr().lock(), renderable),
        }
    }
}

#[cfg(windows)]
fn enable_escapes(terminal: Terminal) -> Terminal {
    let failed = terminal.is_terminal()
        && terminal.color().is_some()
        && anstyle_query::windows::enable_ansi_colors() == Some(false);
    if failed {
        Terminal {
            color: None,
            ..terminal
        }
    } else {
        terminal
    }
}

#[cfg(not(windows))]
fn enable_escapes(terminal: Terminal) -> Terminal {
    terminal
}

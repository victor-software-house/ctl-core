//! The writer: renders a widget at the stream's width and writes it.

use std::error::Error as StdError;
use std::fmt;
use std::io::{self, Write};

use ctl_text::{Context, Renderable};

#[cfg(feature = "detect")]
use crate::color::ColorMode;
use crate::encode::Encoder;
use crate::stream::Stream;
#[cfg(feature = "detect")]
use crate::terminal::DetectOptions;
use crate::terminal::Terminal;

/// Why writing failed.
///
/// A closed pipe is its own variant because it is not a failure of the
/// program. A reader such as `head` that has read enough closes the stream,
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

impl From<Error> for io::Error {
    /// [`Error::Io`] gives back the error it holds, with its message.
    /// [`Error::BrokenPipe`] always converts to an error of
    /// [`io::ErrorKind::BrokenPipe`]. When the error it holds already reports
    /// `BrokenPipe`, as every one [`From<io::Error>`] builds does, it gives
    /// that error back, so the OS error code survives. Otherwise it gives a new
    /// `BrokenPipe` error.
    ///
    /// Nothing is boxed, so a caller that returns [`io::Result`] keeps a closed
    /// pipe apart without carrying this type's formatting code.
    fn from(error: Error) -> Self {
        match error {
            Error::BrokenPipe(error) if error.kind() == io::ErrorKind::BrokenPipe => error,
            Error::BrokenPipe(_) => io::ErrorKind::BrokenPipe.into(),
            Error::Io(error) => error,
        }
    }
}

/// Writes widgets to one output stream, in the colour and at the width
/// decided for that stream.
///
/// A console renders a [`Renderable`] at the stream's width, encodes the lines
/// with [`Encoder`], and writes them. With colour off it writes the plain text
/// of the lines. A stream with no width, because the fallback is off, renders
/// in an [unbounded](Context::unbounded) context, so a widget takes its
/// natural width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Console {
    stream: Stream,
    terminal: Terminal,
}

impl Console {
    /// A console for `stream` that follows the decision in `terminal`.
    #[must_use]
    pub fn new(stream: Stream, terminal: Terminal) -> Self {
        Self { stream, terminal }
    }

    /// A console for `stream` that decides from the process environment and
    /// the stream, under `options`. Needs the `detect` feature.
    #[cfg(feature = "detect")]
    #[must_use]
    pub fn from_process(stream: Stream, mode: ColorMode, options: DetectOptions) -> Self {
        Self::new(stream, Terminal::from_process(stream, mode, options))
    }

    /// The stream this console writes to.
    #[must_use]
    pub fn stream(self) -> Stream {
        self.stream
    }

    /// The colour, depth, and width decided for the stream.
    #[must_use]
    pub fn terminal(self) -> Terminal {
        self.terminal
    }

    /// The text `print` writes for `renderable`, without writing it.
    #[must_use]
    pub fn render(self, renderable: &dyn Renderable) -> String {
        let context = self
            .terminal
            .width()
            .map_or_else(Context::unbounded, Context::new);
        let lines = renderable.render(&context);
        Encoder::new(self.terminal.color()).encode(&lines)
    }

    /// Writes `renderable` to `writer` as `print` would write it to the
    /// stream, then flushes.
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

    /// Writes `renderable` to the stream, then flushes. Needs the `detect`
    /// feature.
    ///
    /// # Errors
    ///
    /// [`Error::BrokenPipe`] when the reader has closed the stream, and
    /// [`Error::Io`] for any other write or flush failure.
    #[cfg(feature = "detect")]
    pub fn print(self, renderable: &dyn Renderable) -> Result<(), Error> {
        match self.stream {
            Stream::Stdout => self.write_to(&mut io::stdout().lock(), renderable),
            Stream::Stderr => self.write_to(&mut io::stderr().lock(), renderable),
        }
    }
}

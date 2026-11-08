//! The output stream a colour and width decision is made for.

use std::io::{self, IsTerminal};

#[cfg(any(unix, windows))]
use terminal_size::{Width, terminal_size_of};

/// A standard output stream.
///
/// Colour and width are decided for the stream the output goes to, so stdout
/// can be a pipe while stderr is a terminal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// What the operating system says about one stream: a pipe or file, or a
/// terminal of some width.
///
/// [`Terminal::from_probe`](crate::Terminal::from_probe) turns a probe into a
/// decision without touching the process, so a test states the probe it
/// wants.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Probe {
    /// A pipe, a file, or a closed stream.
    Pipe,
    /// A terminal. `columns` is its width, when the system reports one.
    Terminal {
        /// The window width in cells, when the system reports a non-zero one.
        columns: Option<u16>,
    },
}

impl Probe {
    /// What the system says about `stream` right now.
    #[must_use]
    pub fn of(stream: Stream) -> Self {
        match stream {
            Stream::Stdout => {
                let handle = io::stdout();
                if handle.is_terminal() {
                    Self::Terminal {
                        columns: columns_of(handle),
                    }
                } else {
                    Self::Pipe
                }
            }
            Stream::Stderr => {
                let handle = io::stderr();
                if handle.is_terminal() {
                    Self::Terminal {
                        columns: columns_of(handle),
                    }
                } else {
                    Self::Pipe
                }
            }
        }
    }

    /// Whether the stream is a terminal.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal { .. })
    }

    /// The terminal's width in cells, when it is a terminal that reports one.
    #[must_use]
    pub fn columns(self) -> Option<u16> {
        match self {
            Self::Pipe => None,
            Self::Terminal { columns } => columns,
        }
    }
}

#[cfg(unix)]
fn columns_of(stream: impl std::os::fd::AsFd) -> Option<u16> {
    terminal_size_of(stream).map(|(Width(columns), _)| columns)
}

#[cfg(windows)]
fn columns_of(stream: impl std::os::windows::io::AsHandle) -> Option<u16> {
    terminal_size_of(stream).map(|(Width(columns), _)| columns)
}

#[cfg(not(any(unix, windows)))]
fn columns_of<T>(_stream: T) -> Option<u16> {
    None
}

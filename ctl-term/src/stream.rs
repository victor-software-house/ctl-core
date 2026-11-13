//! The output stream a colour and width decision is made for.

#[cfg(feature = "detect")]
use std::io::{self, IsTerminal};
#[cfg(all(feature = "detect", unix))]
use std::os::fd::AsFd;

#[cfg(all(feature = "detect", unix))]
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

/// What the operating system says about one stream: redirected away from a
/// terminal, or a terminal of some width.
///
/// [`Terminal::from_probe`](crate::Terminal::from_probe) turns a probe into a
/// decision without touching the process, so a test chooses the probe to
/// use.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Probe {
    /// A pipe, a file, or a closed stream: anything but a terminal.
    Redirected,
    /// A terminal. Build it with [`Probe::terminal`].
    #[non_exhaustive]
    Terminal {
        /// The window width in cells, when the system reports a non-zero one.
        columns: Option<u16>,
    },
}

impl Probe {
    /// A terminal whose window is `columns` cells wide, or whose width the
    /// system does not report, for `None`. A width of 0 counts as not reported.
    #[must_use]
    pub const fn terminal(columns: Option<u16>) -> Self {
        match columns {
            Some(0) => Self::Terminal { columns: None },
            Some(_) | None => Self::Terminal { columns },
        }
    }

    /// What the system says about `stream` right now. Needs the `detect`
    /// feature.
    #[cfg(feature = "detect")]
    #[must_use]
    pub fn of(stream: Stream) -> Self {
        match stream {
            Stream::Stdout => {
                let handle = io::stdout();
                if handle.is_terminal() {
                    Self::terminal(columns_of(handle))
                } else {
                    Self::Redirected
                }
            }
            Stream::Stderr => {
                let handle = io::stderr();
                if handle.is_terminal() {
                    Self::terminal(columns_of(handle))
                } else {
                    Self::Redirected
                }
            }
        }
    }

    /// Whether the stream is a terminal, whether or not it reports a width.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal { .. })
    }

    /// The terminal's width in cells, when it is a terminal that reports one.
    #[must_use]
    pub fn columns(self) -> Option<u16> {
        match self {
            Self::Redirected => None,
            Self::Terminal { columns } => columns,
        }
    }
}

#[cfg(all(feature = "detect", unix))]
fn columns_of(stream: impl AsFd) -> Option<u16> {
    terminal_size_of(stream).map(|(Width(columns), _)| columns)
}

/// Only Unix reports a window width. The crate supports and tests Unix only,
/// so another target gets no width from the window.
#[cfg(all(feature = "detect", not(unix)))]
fn columns_of<T>(_stream: T) -> Option<u16> {
    None
}

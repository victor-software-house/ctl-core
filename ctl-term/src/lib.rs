//! Terminal output for CLIs. It decides whether to write colour and at what
//! depth, how wide the output is, and where an escape sequence is written.
//!
//! The crate works on the plain lines [`ctl_text`] produces, which it
//! re-exports. A [`Terminal`] states what is known about one output stream. An
//! [`Encoder`] turns lines into bytes, adding the styling after layout. A
//! [`Console`] puts the two together: it renders any
//! [`Renderable`](ctl_text::Renderable) at the width of its stream and writes
//! it.
//!
//! # Quick start
//!
//! Decide once for a stream, then print. `from_probe` takes the stream's
//! facts and an environment lookup, so this example needs no terminal; a
//! program calls `Console::from_process` instead and passes
//! [`ColorMode::Auto`].
//!
//! ```
//! use std::collections::HashMap;
//!
//! use ctl_term::ctl_text::anstyle::AnsiColor;
//! use ctl_term::ctl_text::{Style, Text};
//! use ctl_term::{ColorMode, Console, DetectOptions, Probe, Stream, Terminal};
//!
//! let variables = HashMap::from([("FORCE_COLOR", "3")]);
//! let env = |name: &str| variables.get(name).map(|value| (*value).to_owned());
//! let terminal = Terminal::from_probe(
//!     Probe::Redirected,
//!     ColorMode::Auto,
//!     DetectOptions::new(),
//!     &env,
//! );
//! let console = Console::new(Stream::Stdout, terminal);
//!
//! let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
//! let text = Text::styled("failed", red);
//! assert_eq!(console.render(&text), "\u{1b}[31mfailed\u{1b}[0m\n");
//! ```
//!
//! # Colour
//!
//! [`ColorMode::Always`] writes colour and [`ColorMode::Never`] writes none,
//! whatever `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, and
//! `TERM=dumb` hold. They still read the variables that set the depth and the
//! width, `COLORTERM`, `TERM`, `FORCE_COLOR`, and `COLUMNS`. Under
//! [`ColorMode::Auto`] the first rule that applies wins:
//!
//! 1. `NO_COLOR` set and non-empty: no colour.
//! 2. `CLICOLOR_FORCE` set, non-empty, and not `0`: colour.
//! 3. `FORCE_COLOR` set, non-empty, and not `0` or `false`: colour.
//! 4. `CLICOLOR=0`: no colour.
//! 5. `TERM=dumb`: no colour.
//! 6. Otherwise colour only when the stream is a terminal.
//!
//! `FORCE_COLOR=0` and `FORCE_COLOR=false` are neutral: they neither force
//! colour nor turn it off, so rules 4 to 6 still apply. Reading the
//! process environment, a value that is not valid UTF-8 counts as set, so such
//! a `NO_COLOR` still turns colour off.
//!
//! When colour is on, the depth is 24-bit when `COLORTERM` is `truecolor` or
//! `24bit` or `FORCE_COLOR=3`. It is 256 colours when `TERM` contains
//! `256color` or `FORCE_COLOR=2`, and 16 colours otherwise.
//! [`Terminal::with_depth`] overrides it. The [`Encoder`] converts every
//! colour to the nearest colour at that depth. A 24-bit colour on a
//! 16-colour terminal prints as the nearest named colour, and an underline
//! colour is dropped at 16 colours.
//!
//! # Width
//!
//! A width takes the first of these that applies:
//!
//! 1. An explicit width, which is exact.
//! 2. The window of the stream being written.
//! 3. A non-zero `COLUMNS`.
//! 4. The fallback of [`DEFAULT_FALLBACK_WIDTH`], 80.
//!
//! A detected width loses a column buffer of [`DEFAULT_COLUMN_BUFFER`], 2
//! columns, and never drops below [`DEFAULT_MINIMUM_WIDTH`], 20, unless the
//! window itself is narrower. The buffer reads no variable until
//! [`DetectOptions::column_buffer_envs`] names some. [`DetectOptions`]
//! overrides each of these and can turn each off. A width is at least 1.
//!
//! # A closed pipe
//!
//! When the reader closes the stream, as `head` does, a write fails with
//! [`Error::BrokenPipe`], which is a variant of its own. This crate reports
//! it and leaves the exit code to the caller.
//!
//! # Platforms
//!
//! The crate supports and tests Unix only. It has no Windows path: it does
//! not turn on virtual-terminal mode, and it reports no window width on
//! another target. Windows is to come with a Windows CI job.
//!
//! # Features
//!
//! `detect`, on by default, asks the operating system about a stream. It adds
//! `Probe::of`, `Terminal::detect`, the `from_process` constructors, and
//! `Console::print`, and brings in `terminal_size`. Without it the crate holds
//! the [`Encoder`], the colour conversion, and [`Terminal::from_probe`], which
//! need no terminal.

mod color;
mod console;
mod encode;
mod palette;
mod stream;
mod terminal;

pub use color::{ColorDepth, ColorMode};
pub use console::{Console, Error};
pub use ctl_text;
pub use encode::{Encoder, RESET};
pub use palette::{ansi256_to_ansi16, rgb_to_ansi16, rgb_to_ansi256};
pub use stream::{Probe, Stream};
pub use terminal::{
    DEFAULT_COLUMN_BUFFER, DEFAULT_FALLBACK_WIDTH, DEFAULT_MINIMUM_WIDTH, DetectOptions, Terminal,
};

/// The README's examples, compiled and run as doctests.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

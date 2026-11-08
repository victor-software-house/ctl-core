//! Terminal output for CLIs: whether to write colour and at what depth, how
//! wide the output is, and the one place an escape sequence is written.
//!
//! The crate works on the plain lines [`ctl_text`] produces. A [`Terminal`]
//! states what is known about one output stream. An [`Encoder`] turns lines
//! into bytes, adding the styling after layout. A [`Console`] puts the two
//! together: it renders any [`Renderable`](ctl_text::Renderable) at the width
//! of its stream and writes it. The crate depends on `ctl-text`, `anstyle`,
//! and `terminal_size`, and on `anstyle-query` on Windows, and on no widget
//! crate.
//!
//! # Quick start
//!
//! Decide once for a stream, then print. `from_probe` takes the stream's
//! facts and an environment lookup, so this example needs no terminal; a
//! program calls [`Console::from_process`] instead and passes
//! [`ColorMode::Auto`].
//!
//! ```
//! use std::collections::HashMap;
//!
//! use anstyle::{AnsiColor, Style};
//! use ctl_term::{ColorMode, Console, DetectOptions, Probe, Stream, Terminal};
//! use ctl_text::Text;
//!
//! let variables = HashMap::from([("FORCE_COLOR", "3")]);
//! let env = |name: &str| variables.get(name).map(|value| (*value).to_owned());
//! let terminal = Terminal::from_probe(Probe::Pipe, ColorMode::Auto, DetectOptions::new(), &env);
//! let console = Console::new(Stream::Stdout, terminal);
//!
//! let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
//! let text = Text::styled("failed", red);
//! assert_eq!(console.render(&text), "\u{1b}[31mfailed\u{1b}[0m\n");
//! ```
//!
//! # Colour
//!
//! [`ColorMode::Always`] writes colour and [`ColorMode::Never`] writes none;
//! neither reads a variable. Under [`ColorMode::Auto`] the first rule that
//! applies wins:
//!
//! 1. `NO_COLOR` set and non-empty: no colour.
//! 2. `CLICOLOR_FORCE` set, non-empty, and not `0`: colour.
//! 3. `FORCE_COLOR` set, non-empty, and not `0` or `false`: colour.
//! 4. `CLICOLOR=0`: no colour.
//! 5. `TERM=dumb`: no colour.
//! 6. Otherwise colour only when the stream is a terminal.
//!
//! When colour is on, the depth is 24-bit when `COLORTERM` is `truecolor` or
//! `24bit` or `FORCE_COLOR=3`; 256 colours when `TERM` contains `256color` or
//! `FORCE_COLOR=2`; and 16 colours otherwise. [`Terminal::with_depth`]
//! overrides it. The [`Encoder`] converts every colour to the nearest colour
//! at that depth, so a 24-bit colour on a 16-colour terminal prints as the
//! nearest named colour.
//!
//! # Width
//!
//! A width takes the first of these that applies: an explicit width, which is
//! exact; the window of the stream being written; a non-zero `COLUMNS`; the
//! fallback of 80. A detected width loses a buffer of 2 columns, read from
//! `CTL_CORE_COLUMN_BUFFER`, and never drops below 20 unless the window itself
//! is narrower. [`DetectOptions`] overrides each of these and can turn each
//! off.
//!
//! # A closed pipe
//!
//! When the reader closes the stream, as `head` does, a write fails with
//! [`Error::BrokenPipe`], which is a variant of its own. This crate reports
//! it and leaves the exit code to the caller.
//!
//! # Windows
//!
//! A [`Console`] for a terminal turns on virtual-terminal mode through
//! `anstyle-query`. When that fails it writes no colour.

mod color;
mod console;
mod encode;
mod palette;
mod stream;
mod terminal;

pub use color::{ColorDepth, ColorMode};
pub use console::{Console, Error};
pub use encode::{Encoder, RESET};
pub use palette::{ansi256_to_ansi16, rgb_to_ansi16, rgb_to_ansi256};
pub use stream::{Probe, Stream};
pub use terminal::{
    DEFAULT_BUFFER, DEFAULT_BUFFER_ENV, DEFAULT_BUFFER_ENVS, DEFAULT_FALLBACK, DEFAULT_MINIMUM,
    DetectOptions, Terminal,
};

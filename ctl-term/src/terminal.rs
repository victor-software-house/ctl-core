//! Terminal facts: colour, colour depth, and width for one stream.

use crate::color::{ColorDepth, ColorMode};
use crate::stream::Probe;
#[cfg(feature = "detect")]
use crate::stream::Stream;

/// The cells reserved from a detected width, so a line never touches the
/// edge of the window.
pub const DEFAULT_COLUMN_BUFFER: u16 = 2;

/// The width a detected width never shrinks below, unless the window itself is
/// narrower.
pub const DEFAULT_MINIMUM_WIDTH: u16 = 20;

/// The width used when the stream gives none and `COLUMNS` is unset, empty,
/// zero, or not a number from 1 to 65535.
pub const DEFAULT_FALLBACK_WIDTH: u16 = 80;

/// How the width of a stream is decided, and every default it applies.
///
/// A width takes the first of these that applies: an explicit
/// [`width`](Self::width), which is exact; the window of the stream; a
/// `COLUMNS`, when it holds a number from 1 to 65535; the
/// [fallback](Self::fallback_width). The column buffer and the minimum apply
/// only to the window and `COLUMNS`. The width that results is at least 1.
///
/// Each default has an override and a way to turn it off:
///
/// - An explicit width bypasses detection.
/// - A column buffer of 0 reserves nothing.
/// - A minimum of 0 sets no floor.
/// - A fallback of `None` leaves the width undecided.
///
/// The column buffer reads no variable until
/// [`column_buffer_envs`](Self::column_buffer_envs) names some.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DetectOptions {
    width: Option<u16>,
    column_buffer: Option<u16>,
    column_buffer_envs: &'static [&'static str],
    minimum_width: u16,
    fallback_width: Option<u16>,
}

impl DetectOptions {
    /// The defaults: no explicit width, a column buffer of
    /// [`DEFAULT_COLUMN_BUFFER`] read from no variable, a minimum of
    /// [`DEFAULT_MINIMUM_WIDTH`], and a fallback of [`DEFAULT_FALLBACK_WIDTH`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            width: None,
            column_buffer: None,
            column_buffer_envs: &[],
            minimum_width: DEFAULT_MINIMUM_WIDTH,
            fallback_width: Some(DEFAULT_FALLBACK_WIDTH),
        }
    }

    /// Use exactly `columns`, with no detection, buffer, or minimum. 0 counts
    /// as 1.
    #[must_use]
    pub const fn width(mut self, columns: u16) -> Self {
        self.width = Some(columns);
        self
    }

    /// Reserve `columns` from a detected width. 0 reserves nothing. This
    /// replaces whatever the variables of
    /// [`column_buffer_envs`](Self::column_buffer_envs) hold.
    #[must_use]
    pub const fn column_buffer(mut self, columns: u16) -> Self {
        self.column_buffer = Some(columns);
        self
    }

    /// Read the column buffer from the first of `names` that holds a number.
    /// An empty list reads no variable, which is the default.
    #[must_use]
    pub const fn column_buffer_envs(mut self, names: &'static [&'static str]) -> Self {
        self.column_buffer_envs = names;
        self
    }

    /// Never shrink a detected width below `columns`, unless the window is
    /// narrower. 0 sets no floor.
    #[must_use]
    pub const fn minimum_width(mut self, columns: u16) -> Self {
        self.minimum_width = columns;
        self
    }

    /// Use `columns` when the stream gives no width and `COLUMNS` is unset,
    /// empty, zero, or not a number from 1 to 65535.
    /// `None` leaves the width undecided, and 0 counts as 1.
    #[must_use]
    pub const fn fallback_width(mut self, columns: Option<u16>) -> Self {
        self.fallback_width = columns;
        self
    }
}

impl Default for DetectOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// What is known about one output stream: whether it is a terminal, whether
/// to write colour, at what depth, and how wide the output is.
///
/// A terminal is decided once, for the stream the output goes to, before
/// anything renders. The decision reads an environment lookup passed by the
/// caller, so a test hands it a map and touches no process state.
///
/// # Colour
///
/// The Colour section of the [crate docs](crate) states the order and the depth
/// rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Terminal {
    pub(crate) attached: bool,
    pub(crate) color: Option<ColorDepth>,
    pub(crate) width: Option<u16>,
}

impl Terminal {
    /// What `stream` is, reading variables through `env`.
    ///
    /// `env` returns a variable's value, or `None` when it is unset. It asks
    /// the stream when called, so two calls can differ after a resize or a
    /// redirect.
    /// Use [`from_probe`](Self::from_probe) to state the stream. Needs the
    /// `detect` feature.
    #[cfg(feature = "detect")]
    #[must_use]
    pub fn detect(
        stream: Stream,
        mode: ColorMode,
        options: DetectOptions,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Self {
        Self::from_probe(Probe::of(stream), mode, options, env)
    }

    /// [`detect`](Self::detect) reading the process environment. A value that
    /// is not valid UTF-8 counts as set, with its invalid bytes replaced.
    /// Needs the `detect` feature.
    #[cfg(feature = "detect")]
    #[must_use]
    pub fn from_process(stream: Stream, mode: ColorMode, options: DetectOptions) -> Self {
        Self::detect(stream, mode, options, &process_env)
    }

    /// The decision for a stream that `probe` describes.
    ///
    /// This is the whole decision as a function of its arguments. The other
    /// constructors call it after they probe a stream.
    #[must_use]
    pub fn from_probe(
        probe: Probe,
        mode: ColorMode,
        options: DetectOptions,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Self {
        let lookup = Lookup(env);
        let color = color_is_on(probe, mode, &lookup).then(|| detect_depth(&lookup));
        Self {
            attached: probe.is_terminal(),
            color,
            width: detect_width(probe, options, &lookup),
        }
    }

    /// This terminal at `depth`, when colour is on.
    ///
    /// The depth otherwise follows `COLORTERM`, `TERM`, and `FORCE_COLOR`.
    /// A terminal with colour off stays off: the mode decides whether to write
    /// colour, and this only decides how many colours.
    #[must_use]
    pub fn at_depth(self, depth: ColorDepth) -> Self {
        Self {
            color: self.color.map(|_| depth),
            ..self
        }
    }

    /// Whether the stream is attached to a terminal, as opposed to a pipe or
    /// a file.
    #[must_use]
    pub fn is_attached(self) -> bool {
        self.attached
    }

    /// The colour depth, or `None` when no colour is written.
    #[must_use]
    pub fn color(self) -> Option<ColorDepth> {
        self.color
    }

    /// The width to lay out to, in cells and at least 1, or `None` when the
    /// stream gives none, `COLUMNS` holds no number from 1 to 65535, and the
    /// fallback is off.
    #[must_use]
    pub fn width(self) -> Option<u16> {
        self.width
    }
}

/// The process variable `name`, with bytes that are not UTF-8 replaced, so a
/// variable holding them still counts as set.
#[cfg(feature = "detect")]
fn process_env(name: &str) -> Option<String> {
    std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

/// An environment lookup that treats an empty value as unset.
struct Lookup<'a>(&'a dyn Fn(&str) -> Option<String>);

impl Lookup<'_> {
    fn get(&self, name: &str) -> Option<String> {
        (self.0)(name).filter(|value| !value.is_empty())
    }

    fn is(&self, name: &str, expected: &str) -> bool {
        self.get(name).is_some_and(|value| value == expected)
    }
}

fn color_is_on(probe: Probe, mode: ColorMode, env: &Lookup<'_>) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => auto_color(probe, env),
    }
}

fn auto_color(probe: Probe, env: &Lookup<'_>) -> bool {
    if env.get("NO_COLOR").is_some() {
        return false;
    }
    if env.get("CLICOLOR_FORCE").is_some_and(|value| value != "0") {
        return true;
    }
    if env
        .get("FORCE_COLOR")
        .is_some_and(|value| value != "0" && value != "false")
    {
        return true;
    }
    if env.is("CLICOLOR", "0") || env.is("TERM", "dumb") {
        return false;
    }
    probe.is_terminal()
}

fn detect_depth(env: &Lookup<'_>) -> ColorDepth {
    let truecolor = env.is("COLORTERM", "truecolor")
        || env.is("COLORTERM", "24bit")
        || env.is("FORCE_COLOR", "3");
    let palette = env
        .get("TERM")
        .is_some_and(|term| term.contains("256color"))
        || env.is("FORCE_COLOR", "2");
    if truecolor {
        ColorDepth::TrueColor
    } else if palette {
        ColorDepth::Ansi256
    } else {
        ColorDepth::Ansi16
    }
}

fn detect_width(probe: Probe, options: DetectOptions, env: &Lookup<'_>) -> Option<u16> {
    let columns = options.width.or_else(|| {
        let detected = probe.columns().or_else(|| {
            env.get("COLUMNS")?
                .parse::<u16>()
                .ok()
                .filter(|columns| *columns >= 1)
        });
        match detected {
            Some(columns) => Some(effective_width(
                columns,
                column_buffer(options, env),
                options.minimum_width,
            )),
            None => options.fallback_width,
        }
    });
    columns.map(|columns| columns.max(1))
}

fn column_buffer(options: DetectOptions, env: &Lookup<'_>) -> u16 {
    options
        .column_buffer
        .or_else(|| {
            options
                .column_buffer_envs
                .iter()
                .filter_map(|name| env.get(name))
                .find_map(|raw| raw.parse().ok())
        })
        .unwrap_or(DEFAULT_COLUMN_BUFFER)
}

fn effective_width(width: u16, buffer: u16, minimum: u16) -> u16 {
    width.saturating_sub(buffer).max(minimum.min(width))
}

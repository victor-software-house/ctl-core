//! Helpers and fixtures shared by the test modules.

use ctl_term::{ColorMode, Console, DetectOptions, Probe, Stream, Terminal};
use ctl_text::anstyle::{AnsiColor, Style};
use ctl_text::{Context, Line, Measurement, Renderable, Span};

/// The escape byte, which only the encoder may write.
pub(crate) const ESC: char = '\u{1b}';

/// A pipe, as stdout is under `| head`.
pub(crate) const PIPE: Probe = Probe::Pipe;

/// A terminal 100 columns wide.
pub(crate) const TERMINAL: Probe = Probe::Terminal { columns: Some(100) };

/// Variable names and values, as a test states an environment.
pub(crate) type Variables = &'static [(&'static str, &'static str)];

/// The lookup for `variables`: a name not listed is unset.
pub(crate) fn lookup(variables: Variables) -> impl Fn(&str) -> Option<String> {
    move |name| {
        variables
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
    }
}

/// The decision for a stream that `probe` describes under `mode`.
pub(crate) fn terminal(probe: Probe, mode: ColorMode, variables: Variables) -> Terminal {
    Terminal::from_probe(probe, mode, DetectOptions::new(), &lookup(variables))
}

/// A stdout console for `probe`, `mode`, and `variables`.
pub(crate) fn console(probe: Probe, mode: ColorMode, variables: Variables) -> Console {
    Console::new(Stream::Stdout, terminal(probe, mode, variables))
}

/// Lines a test hands to the console as they are.
#[derive(Debug)]
pub(crate) struct Rows(pub(crate) Vec<Line>);

impl Renderable for Rows {
    fn measure(&self, _: &Context) -> Measurement {
        Measurement::exact(self.0.iter().map(Line::width).max().unwrap_or(0))
    }

    fn render(&self, _: &Context) -> Vec<Line> {
        self.0.clone()
    }
}

/// A line of spans, each with its style.
pub(crate) fn line(spans: &[(&str, Style)]) -> Line {
    let mut line = Line::new();
    for (text, style) in spans {
        line.push(Span::new(text, *style));
    }
    line
}

/// A foreground colour.
pub(crate) fn fg(color: impl Into<ctl_text::anstyle::Color>) -> Style {
    Style::new().fg_color(Some(color.into()))
}

/// The red every colour test writes.
pub(crate) fn red() -> Style {
    fg(AnsiColor::Red)
}

/// The width of the widest line of `text`, in cells.
pub(crate) fn widest(text: &str) -> u16 {
    text.lines().map(ctl_text::text_width).max().unwrap_or(0)
}

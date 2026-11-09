//! The `terminal-output` spec scenarios that this crate owns, each through
//! the `Console` that renders and encodes. The scenarios that need ctl-core's
//! `App` or its `--color` flag run here with the matching `ColorMode` and
//! variables.

use ctl_term::{ColorDepth, ColorMode, Console, DetectOptions, Error, RESET, Stream, Terminal};
use ctl_text::anstyle::{AnsiColor, RgbColor};
use ctl_text::{Line, Text};

use crate::common::{ESC, PIPE, Rows, TERMINAL, console, fg, line, red, widest};

const PARAGRAPH: &str = "the quick brown fox jumps over the lazy dog and keeps running \
    through the field until the sun goes down behind the hills";

fn coloured_rows() -> Rows {
    let rows = (0..10)
        .map(|row| {
            line(&[
                (format!("QCTL-{row:03}").as_str(), red()),
                ("  a row title", ctl_text::Style::new()),
            ])
        })
        .collect();
    Rows(rows)
}

fn has_sgr(text: &str) -> bool {
    text.contains("\u{1b}[")
}

/// Whether the last escape sequence in `line` is not a reset, so a style is
/// still open at its end.
fn leaves_style_open(line: &str) -> bool {
    line.rfind(ESC)
        .is_some_and(|start| !line[start..].starts_with(RESET))
}

#[test]
fn never_beats_a_forcing_variable() {
    let console = console(TERMINAL, ColorMode::Never, &[("CLICOLOR_FORCE", "1")]);
    assert!(!console.render(&coloured_rows()).contains(ESC));
}

#[test]
fn always_beats_no_color() {
    let console = console(PIPE, ColorMode::Always, &[("NO_COLOR", "1")]);
    assert!(has_sgr(&console.render(&coloured_rows())));
}

#[test]
fn no_color_wins_over_clicolor_force() {
    let variables = &[("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")];
    let console = console(TERMINAL, ColorMode::Auto, variables);
    assert!(!console.render(&coloured_rows()).contains(ESC));
}

#[test]
fn forced_colour_into_a_pipe() {
    let console = console(PIPE, ColorMode::Auto, &[("FORCE_COLOR", "3")]);
    assert!(has_sgr(&console.render(&coloured_rows())));
}

#[test]
fn force_color_zero_is_not_a_force() {
    let console = console(PIPE, ColorMode::Auto, &[("FORCE_COLOR", "0")]);
    assert!(!console.render(&coloured_rows()).contains(ESC));
}

#[test]
fn force_color_zero_turns_nothing_off() {
    let console = console(TERMINAL, ColorMode::Auto, &[("FORCE_COLOR", "0")]);
    assert!(has_sgr(&console.render(&coloured_rows())));
}

#[test]
fn always_still_follows_the_depth_variable() {
    let console = console(PIPE, ColorMode::Always, &[("COLORTERM", "truecolor")]);
    let text = Text::styled("alert", fg(RgbColor(255, 0, 0)));
    assert_eq!(
        console.render(&text),
        "\u{1b}[38;2;255;0;0malert\u{1b}[0m\n"
    );
}

#[test]
fn never_still_follows_columns() {
    let console = console(PIPE, ColorMode::Never, &[("COLUMNS", "100")]);
    assert_eq!(console.terminal().width(), Some(98));
}

#[test]
fn a_pipe_with_no_variable_gets_no_colour() {
    let console = console(PIPE, ColorMode::Auto, &[]);
    assert!(!console.render(&coloured_rows()).contains(ESC));
}

#[test]
fn colour_is_decided_per_stream() {
    let stdout = Console::new(
        Stream::Stdout,
        Terminal::from_probe(PIPE, ColorMode::Auto, DetectOptions::new(), &|_| None),
    );
    let stderr = Console::new(
        Stream::Stderr,
        Terminal::from_probe(TERMINAL, ColorMode::Auto, DetectOptions::new(), &|_| None),
    );
    assert!(!stdout.render(&coloured_rows()).contains(ESC));
    assert!(has_sgr(&stderr.render(&coloured_rows())));
    assert_eq!(stdout.stream(), Stream::Stdout);
    assert_eq!(stderr.stream(), Stream::Stderr);
}

#[test]
fn a_24_bit_colour_on_a_16_colour_terminal_is_a_named_colour() {
    let console = console(PIPE, ColorMode::Always, &[("TERM", "xterm")]);
    assert_eq!(console.terminal().color(), Some(ColorDepth::Ansi16));
    let text = Text::styled("alert", fg(RgbColor(255, 0, 0)));
    assert_eq!(console.render(&text), "\u{1b}[91malert\u{1b}[0m\n");
}

#[test]
fn a_colour_is_converted_for_each_depth_the_environment_names() {
    let text = Text::styled("alert", fg(RgbColor(255, 0, 0)));
    let cases: [(&[(&str, &str)], &str); 3] = [
        (
            &[("COLORTERM", "truecolor")],
            "\u{1b}[38;2;255;0;0malert\u{1b}[0m\n",
        ),
        (
            &[("TERM", "xterm-256color")],
            "\u{1b}[38;5;196malert\u{1b}[0m\n",
        ),
        (&[("TERM", "xterm")], "\u{1b}[91malert\u{1b}[0m\n"),
    ];
    for (variables, expected) in cases {
        let console = console(PIPE, ColorMode::Always, variables);
        assert_eq!(console.render(&text), expected, "{variables:?}");
    }
}

#[test]
fn a_coloured_table_cut_by_head_leaves_no_style_open() {
    let console = console(PIPE, ColorMode::Always, &[]);
    let output = console.render(&coloured_rows());
    let kept: Vec<&str> = output.lines().take(3).collect();
    assert_eq!(kept.len(), 3);
    for kept_line in kept {
        assert!(has_sgr(kept_line), "{kept_line:?}");
        assert!(!leaves_style_open(kept_line), "{kept_line:?}");
    }
}

#[test]
fn captured_output_under_auto_holds_no_escape() {
    let console = console(PIPE, ColorMode::Auto, &[]);
    let mixed = Rows(vec![line(&[("x", red())]), Line::new()]);
    assert!(!console.render(&mixed).contains(ESC));
}

#[test]
fn output_piped_with_columns_set_wraps_to_the_buffered_width() {
    let console = console(PIPE, ColorMode::Never, &[("COLUMNS", "100")]);
    assert_eq!(console.terminal().width(), Some(98));
    let paragraph = Text::plain(PARAGRAPH.repeat(4));
    let output = console.render(&paragraph);
    assert!(widest(&output) <= 98, "{output}");
    assert!(widest(&output) >= 85, "{output}");
}

#[test]
fn nothing_to_detect_lays_out_to_80_cells() {
    let console = console(PIPE, ColorMode::Never, &[]);
    assert_eq!(console.terminal().width(), Some(80));
    let paragraph = Text::plain(PARAGRAPH.repeat(4));
    let output = console.render(&paragraph);
    assert!(widest(&output) <= 80, "{output}");
    assert!(widest(&output) >= 70, "{output}");
}

#[test]
fn the_width_of_a_terminal_beats_columns() {
    let console = console(TERMINAL, ColorMode::Never, &[("COLUMNS", "60")]);
    assert_eq!(console.terminal().width(), Some(98));
}

#[test]
fn a_stream_with_no_width_renders_at_the_natural_width() {
    let options = DetectOptions::new().fallback_width(None);
    let terminal = Terminal::from_probe(PIPE, ColorMode::Never, options, &|_| None);
    let console = Console::new(Stream::Stdout, terminal);
    let paragraph = Text::plain(PARAGRAPH);
    assert_eq!(console.render(&paragraph), format!("{PARAGRAPH}\n"));
}

#[test]
fn a_closed_pipe_is_its_own_error() {
    use std::io::{self, ErrorKind, Write};

    struct Failing(ErrorKind);

    impl Write for Failing {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let console = console(PIPE, ColorMode::Never, &[]);
    let text = Text::plain("row");
    let closed = console.write_to(&mut Failing(ErrorKind::BrokenPipe), &text);
    assert!(matches!(closed, Err(Error::BrokenPipe(_))), "{closed:?}");
    let other = console.write_to(&mut Failing(ErrorKind::PermissionDenied), &text);
    assert!(matches!(other, Err(Error::Io(_))), "{other:?}");
}

#[test]
fn an_error_becomes_an_io_error_of_the_same_kind() {
    use std::io::{self, ErrorKind};

    const EPIPE: i32 = 32;
    let closed = io::Error::from(Error::from(io::Error::from_raw_os_error(EPIPE)));
    assert_eq!(closed.kind(), ErrorKind::BrokenPipe);
    assert_eq!(closed.raw_os_error(), Some(EPIPE));
    let built = io::Error::from(Error::BrokenPipe(io::Error::other("not a pipe")));
    assert_eq!(built.kind(), ErrorKind::BrokenPipe);
    let full = io::Error::from(Error::Io(io::Error::other("disk full")));
    assert_eq!(full.kind(), ErrorKind::Other);
    assert_eq!(full.to_string(), "disk full");
}

#[test]
fn named_colours_are_the_same_at_every_depth() {
    let text = Text::styled("ok", fg(AnsiColor::Green));
    let expected = "\u{1b}[32mok\u{1b}[0m\n";
    for variables in [
        &[("COLORTERM", "truecolor")][..],
        &[("TERM", "xterm-256color")][..],
        &[][..],
    ] {
        let console = console(PIPE, ColorMode::Always, variables);
        assert_eq!(console.render(&text), expected, "{variables:?}");
    }
}

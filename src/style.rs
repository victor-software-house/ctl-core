//! ANSI styles for pretty views.

use anstyle::{AnsiColor, Style};

/// Section heading (cyan bold).
pub const HEADING: Style = Style::new()
    .bold()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Cyan)));
/// Success text (green bold).
pub const SUCCESS: Style = Style::new()
    .bold()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)));
/// Warning text (yellow bold).
pub const WARNING: Style = Style::new()
    .bold()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Yellow)));
/// Error text (red bold).
pub const ERROR: Style = Style::new()
    .bold()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Red)));
/// Value / metavar text (yellow bold).
pub const VALUE: Style = Style::new()
    .bold()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Yellow)));
/// Secondary text (bright black).
pub const MUTED: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::BrightBlack)));
/// Flag text (green bold).
pub const OPTION: Style = Style::new()
    .bold()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)));

#[must_use]
/// Wrap `value` in `style` and reset.
pub fn styled(style: Style, value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format!("{}{value}{}", style.render(), style.render_reset())
    }
}

#[cfg(test)]
mod tests {
    use super::{OPTION, styled};

    #[test]
    fn empty_stays_empty() {
        assert_eq!(styled(OPTION, ""), "");
    }

    #[test]
    fn wraps_nonempty() {
        let out = styled(OPTION, "--help");
        assert!(out.contains("--help"));
        assert_ne!(out, "--help");
    }
}

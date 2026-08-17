//! Compact pretty tables for command output.

use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, ContentArrangement, Table};

use crate::color::ColorMode;
use crate::style::{HEADING, OPTION, styled};

/// Two-column token / value table. Tokens are styled unless `color` is never.
#[must_use]
pub fn kv(
    color: ColorMode,
    rows: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
) -> String {
    let cells = rows
        .into_iter()
        .map(|(token, value)| vec![token.as_ref().to_owned(), value.as_ref().to_owned()]);
    render(color, None, cells, true)
}

/// Headered grid. Styled unless `color` is never.
#[must_use]
pub fn grid(
    color: ColorMode,
    headers: &[&str],
    rows: impl IntoIterator<Item = Vec<String>>,
) -> String {
    render(color, Some(headers), rows, true)
}

fn paint(color: ColorMode, style: anstyle::Style, value: &str) -> String {
    if color == ColorMode::Never {
        value.to_owned()
    } else {
        styled(style, value)
    }
}

fn render(
    color: ColorMode,
    headers: Option<&[&str]>,
    rows: impl IntoIterator<Item = Vec<String>>,
    token_first: bool,
) -> String {
    let mut table = Table::new();
    table
        .load_style(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic);
    if let Some(headers) = headers {
        table.set_header(
            headers
                .iter()
                .map(|header| Cell::new(paint(color, HEADING, header))),
        );
    }
    for row in rows {
        let cells = row.into_iter().enumerate().map(|(index, cell)| {
            if token_first && index == 0 {
                Cell::new(paint(color, OPTION, &cell))
            } else {
                Cell::new(cell)
            }
        });
        table.add_row(cells);
    }
    format!("{table}")
}

#[cfg(test)]
mod tests {
    use super::{grid, kv};
    use crate::color::ColorMode;
    use crate::style::OPTION;

    #[test]
    fn kv_is_a_table_not_spaces() {
        let out = kv(
            ColorMode::Never,
            [("crate", "demo@0.0.1"), ("package", "@org/pkg@0.0.1")],
        );
        assert!(out.contains("crate"), "{out}");
        assert!(out.contains("demo@0.0.1"), "{out}");
        assert!(out.contains("package"), "{out}");
        assert!(out.contains('│') || out.contains('|'), "{out}");
        assert!(!out.contains(&OPTION.render().to_string()), "{out}");
    }

    #[test]
    fn never_has_no_ansi() {
        let out = kv(ColorMode::Never, [("release", "v0.0.1")]);
        assert!(!out.contains('\u{1b}'), "{out:?}");
        assert!(out.contains("release"), "{out}");
    }

    #[test]
    fn grid_has_headers() {
        let out = grid(
            ColorMode::Never,
            &["id", "runner"],
            [vec!["linux-x64".into(), "ubuntu-latest".into()]],
        );
        assert!(out.contains("linux-x64"), "{out}");
        assert!(out.contains("ubuntu-latest"), "{out}");
        assert!(out.contains("id"), "{out}");
    }
}

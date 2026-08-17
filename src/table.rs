//! Compact pretty tables for command output.

use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, ContentArrangement, Table};

use crate::style::{HEADING, OPTION, styled};

/// Two-column token / value table. The token is styled.
#[must_use]
pub fn kv(rows: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>) -> String {
    let cells = rows
        .into_iter()
        .map(|(token, value)| vec![token.as_ref().to_owned(), value.as_ref().to_owned()]);
    render(None, cells, true)
}

/// Headered grid. Headers are styled; the first column is a token.
#[must_use]
pub fn grid(headers: &[&str], rows: impl IntoIterator<Item = Vec<String>>) -> String {
    render(Some(headers), rows, true)
}

fn render(
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
                .map(|header| Cell::new(styled(HEADING, header))),
        );
    }
    for row in rows {
        let cells = row.into_iter().enumerate().map(|(index, cell)| {
            if token_first && index == 0 {
                Cell::new(styled(OPTION, &cell))
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
    use crate::style::OPTION;

    #[test]
    fn kv_is_a_table_not_spaces() {
        let out = kv([("crate", "demo@0.0.1"), ("package", "@org/pkg@0.0.1")]);
        assert!(out.contains("crate"), "{out}");
        assert!(out.contains("demo@0.0.1"), "{out}");
        assert!(out.contains("package"), "{out}");
        assert!(out.contains('│') || out.contains('|'), "{out}");
        assert!(out.contains(&OPTION.render().to_string()), "{out}");
    }

    #[test]
    fn grid_has_headers() {
        let out = grid(
            &["id", "runner"],
            [vec!["linux-x64".into(), "ubuntu-latest".into()]],
        );
        assert!(out.contains("linux-x64"), "{out}");
        assert!(out.contains("ubuntu-latest"), "{out}");
        assert!(out.contains("id"), "{out}");
    }
}

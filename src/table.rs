//! Compatibility helpers over the semantic document renderer.
//!
//! New consumers should compose [`Fields`](crate::Fields) and
//! [`Table`](crate::Table) inside a [`Document`](crate::Document).

use crate::color::ColorMode;
use crate::document::{Document, Fields, Table};
use crate::render::RenderOptions;

/// Two-column token/value fields. Tokens are styled unless `color` is never.
#[must_use]
pub fn kv(
    color: ColorMode,
    rows: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
) -> String {
    let fields = rows
        .into_iter()
        .fold(Fields::new(), |fields, (token, value)| {
            fields.row(token.as_ref(), value.as_ref())
        });
    Document::new()
        .fields(fields)
        .render(RenderOptions::new(color))
        .trim_end()
        .to_owned()
}

/// Headered grid. Headers and the first column carry semantic styles.
#[must_use]
pub fn grid(
    color: ColorMode,
    headers: &[&str],
    rows: impl IntoIterator<Item = Vec<String>>,
) -> String {
    let table = rows.into_iter().fold(
        Table::new(headers.iter().copied()).token_column(0),
        Table::row,
    );
    Document::new()
        .table(table)
        .render(RenderOptions::new(color))
        .trim_end()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{grid, kv};
    use crate::color::ColorMode;

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
        assert!(!out.contains('\u{1b}'), "{out:?}");
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

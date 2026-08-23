//! Private terminal width detection for the semantic renderer.

use comfy_table::Table;

const MIN_WIDTH: u16 = 20;

/// Detected TTY width, or `COLUMNS` when at least 20.
pub(crate) fn terminal_width() -> Option<u16> {
    Table::new().width().or_else(|| {
        std::env::var("COLUMNS")
            .ok()?
            .parse::<u16>()
            .ok()
            .filter(|width| *width >= MIN_WIDTH)
    })
}

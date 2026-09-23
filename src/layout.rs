//! Private terminal width detection for the semantic renderer.

use comfy_table::Table;

/// Detected TTY width, or any nonzero `COLUMNS`, minus the buffer without
/// dropping below the configured minimum unless the terminal itself is
/// narrower.
pub(crate) fn terminal_width(buffer: u16, minimum: u16) -> Option<u16> {
    detected_width().map(|width| effective_width(width, buffer, minimum))
}

pub(crate) fn column_buffer(names: &[&str]) -> Option<u16> {
    configured_buffer(names, |name| std::env::var(name).ok())
}

fn detected_width() -> Option<u16> {
    Table::new().width().or_else(|| {
        std::env::var("COLUMNS")
            .ok()?
            .parse::<u16>()
            .ok()
            .filter(|width| *width >= 1)
    })
}

fn effective_width(width: u16, buffer: u16, minimum: u16) -> u16 {
    width.saturating_sub(buffer).max(minimum.min(width)).max(1)
}

fn configured_buffer(names: &[&str], value: impl Fn(&str) -> Option<String>) -> Option<u16> {
    names
        .iter()
        .filter_map(|name| value(name))
        .find_map(|raw| raw.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::{configured_buffer, effective_width};

    #[test]
    fn automatic_width_reserves_the_buffer_above_the_minimum() {
        assert_eq!(effective_width(80, 1, 20), 79);
        assert_eq!(effective_width(80, 3, 20), 77);
        assert_eq!(effective_width(20, 3, 20), 20);
        assert_eq!(effective_width(15, 1, 20), 15);
        assert_eq!(effective_width(80, 0, 20), 80);
        assert_eq!(effective_width(12, 3, 0), 9);
        assert_eq!(effective_width(1, 1, 0), 1);
        let default = crate::render::DEFAULT_COLUMN_BUFFER;
        assert_eq!(effective_width(119, default, 20), 117);
    }

    #[test]
    fn environment_names_are_ordered_aliases() {
        let names = ["PRIMARY", "ALIAS"];
        let found = configured_buffer(&names, |name| match name {
            "PRIMARY" => Some("invalid".to_owned()),
            "ALIAS" => Some("4".to_owned()),
            _ => None,
        });
        assert_eq!(found, Some(4));
    }
}

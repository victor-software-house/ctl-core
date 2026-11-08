//! Baseline files: compare one render with its stored text, and write the
//! text when asked.
//!
//! A baseline is a plain file rather than an insta snapshot. The render-engine
//! change keeps these files frozen as the 0.6.x reference and adds the new
//! engine's outputs beside them, so nothing here may rewrite or review them
//! on its own.

use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The variable that makes [`check`] write a missing or changed baseline
/// instead of reporting it.
const UPDATE: &str = "PARITY_UPDATE";

fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/parity/baseline")
}

fn updating() -> bool {
    std::env::var_os(UPDATE).is_some_and(|value| !value.is_empty())
}

/// Compare `actual` with the baseline file `name`.
///
/// Returns `None` when they match, or when [`UPDATE`] is set and the file was
/// written. Otherwise returns one report: the file is missing, or a line
/// diff of what is stored against what rendered.
pub(crate) fn check(name: &str, actual: &str) -> Option<String> {
    let path = directory().join(name);
    let stored = match fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => panic!("read {}: {error}", path.display()),
    };
    if stored.as_deref() == Some(actual) {
        return None;
    }
    if updating() {
        fs::create_dir_all(directory())
            .unwrap_or_else(|error| panic!("create {}: {error}", directory().display()));
        fs::write(&path, actual)
            .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
        return None;
    }
    Some(match stored {
        None => format!("{name}: no baseline; write it with {UPDATE}=1"),
        Some(stored) => format!(
            "{name}: differs from its baseline\n{}",
            diff(&stored, actual)
        ),
    })
}

/// The stored and rendered text side by side, one marked line each. A line
/// that matches by position has two spaces, the stored line `-`, the
/// rendered line `+`. Lines are split on `\n` so a missing final newline
/// shows as a changed last line.
fn diff(stored: &str, actual: &str) -> String {
    let stored = stored.split('\n').collect::<Vec<_>>();
    let actual = actual.split('\n').collect::<Vec<_>>();
    let mut report = String::new();
    for index in 0..stored.len().max(actual.len()) {
        match (stored.get(index), actual.get(index)) {
            (Some(left), Some(right)) if left == right => {
                let _ = writeln!(report, "  {left}");
            }
            (left, right) => {
                if let Some(left) = left {
                    let _ = writeln!(report, "- {left}");
                }
                if let Some(right) = right {
                    let _ = writeln!(report, "+ {right}");
                }
            }
        }
    }
    report
}

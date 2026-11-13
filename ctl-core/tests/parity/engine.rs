//! Engine files: compare one render with its stored text, write the text
//! when asked, and check that every engine file has its 0.6.x baseline.
//!
//! An engine file is a plain file rather than an insta snapshot, so it sits
//! beside the baseline of the same name and a plain diff of the two
//! directories shows every change from the comfy-table renderer. The
//! baselines in `baseline/` are frozen: nothing here writes them.

use std::fmt::Write as _;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// The variable that makes [`check`] write a missing or changed engine file
/// instead of reporting it.
const UPDATE: &str = "PARITY_UPDATE";

fn parity() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/parity")
}

/// Where the engine files live.
fn directory() -> PathBuf {
    parity().join("engine")
}

/// Where the frozen 0.6.x baselines live.
fn baselines() -> PathBuf {
    parity().join("baseline")
}

fn updating() -> bool {
    env::var_os(UPDATE).is_some_and(|value| !value.is_empty())
}

/// Compare `actual` with the engine file `name`.
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
        None => format!("{name}: no engine file; write it with {UPDATE}=1"),
        Some(stored) => format!(
            "{name}: differs from its engine file\n{}",
            diff(&stored, actual)
        ),
    })
}

/// One report per engine file with no baseline of the same name, so every
/// engine render is compared with a 0.6.x one.
///
/// A missing directory holds no engine file, so it reports nothing: the
/// first `PARITY_UPDATE=1` run creates it while this test may already be
/// running, and a case with no engine file fails its own comparison.
pub(crate) fn without_baseline() -> Vec<String> {
    let Ok(entries) = fs::read_dir(directory()) else {
        return Vec::new();
    };
    let mut reports = entries
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("read {}: {error}", directory().display()))
                .file_name()
        })
        .filter(|name| !baselines().join(name).is_file())
        .map(|name| format!("{}: no baseline of the same name", name.to_string_lossy()))
        .collect::<Vec<_>>();
    reports.sort();
    reports
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

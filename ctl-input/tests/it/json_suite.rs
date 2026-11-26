//! The reader against `JSONTestSuite`, vendored in
//! `tests/fixtures/json-test-suite/`: every file is opened the way a CLI opens
//! it and read as a typeless value, and the outcome is held to a verdict.
//!
//! A `y_` file reads, except the two that write a key twice, which a strict
//! reading refuses. An `n_` file is refused. An `i_` file has the verdict its
//! row gives, and a file with none fails the test, as does a row with no file.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ctl_input::format::{Fault, JSON, Reading};
use ctl_input::{Format, Input, Limits};
use serde::de::IgnoredAny;

/// What reading a file must give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Reads,
    Refused,
    /// Reads when `max_depth` allows this many levels, and is refused under
    /// the default limit.
    ReadsFromDepth(usize),
}

/// The `y_` files a strict reading refuses: each writes a key twice.
const DUPLICATE_KEYS: [&str; 2] = [
    "y_object_duplicated_key",
    "y_object_duplicated_key_and_value",
];

/// The verdict of each `i_` file, by name without its extension.
const IMPLEMENTATION_DEFINED: [(&str, Verdict); 35] = [
    // The reader keeps a number as written, and the field that reads it
    // decides whether it fits.
    ("i_number_double_huge_neg_exp", Verdict::Reads),
    ("i_number_huge_exp", Verdict::Reads),
    ("i_number_neg_int_huge_exp", Verdict::Reads),
    ("i_number_pos_double_huge_exp", Verdict::Reads),
    ("i_number_real_neg_overflow", Verdict::Reads),
    ("i_number_real_pos_overflow", Verdict::Reads),
    ("i_number_real_underflow", Verdict::Reads),
    ("i_number_too_big_neg_int", Verdict::Reads),
    ("i_number_too_big_pos_int", Verdict::Reads),
    ("i_number_very_big_negative_int", Verdict::Reads),
    // A Rust string cannot hold a lone or inverted surrogate.
    ("i_object_key_lone_2nd_surrogate", Verdict::Refused),
    ("i_string_1st_surrogate_but_2nd_missing", Verdict::Refused),
    ("i_string_1st_valid_surrogate_2nd_invalid", Verdict::Refused),
    (
        "i_string_incomplete_surrogate_and_escape_valid",
        Verdict::Refused,
    ),
    ("i_string_incomplete_surrogate_pair", Verdict::Refused),
    (
        "i_string_incomplete_surrogates_escape_valid",
        Verdict::Refused,
    ),
    ("i_string_invalid_lonely_surrogate", Verdict::Refused),
    ("i_string_invalid_surrogate", Verdict::Refused),
    ("i_string_inverted_surrogates_U+1D11E", Verdict::Refused),
    ("i_string_lone_second_surrogate", Verdict::Refused),
    // A file that is not UTF-8 is refused when it is opened.
    ("i_string_UTF-8_invalid_sequence", Verdict::Refused),
    ("i_string_UTF8_surrogate_U+D800", Verdict::Refused),
    ("i_string_invalid_utf-8", Verdict::Refused),
    ("i_string_iso_latin_1", Verdict::Refused),
    ("i_string_lone_utf8_continuation_byte", Verdict::Refused),
    ("i_string_not_in_unicode_range", Verdict::Refused),
    ("i_string_overlong_sequence_2_bytes", Verdict::Refused),
    ("i_string_overlong_sequence_6_bytes", Verdict::Refused),
    ("i_string_overlong_sequence_6_bytes_null", Verdict::Refused),
    ("i_string_truncated-utf-8", Verdict::Refused),
    // UTF-16 is no UTF-8 either.
    ("i_string_UTF-16LE_with_BOM", Verdict::Refused),
    ("i_string_utf16BE_no_BOM", Verdict::Refused),
    ("i_string_utf16LE_no_BOM", Verdict::Refused),
    // RFC 8259 section 8.1 lets a reader skip a UTF-8 byte-order mark.
    ("i_structure_UTF-8_BOM_empty_object", Verdict::Reads),
    (
        "i_structure_500_nested_arrays",
        Verdict::ReadsFromDepth(500),
    ),
];

fn suite() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/json-test-suite/test_parsing")
}

/// The verdict the suite and the table give `name`, or `None` for a file with
/// none.
fn verdict(name: &str) -> Option<Verdict> {
    if let Some((_, verdict)) = IMPLEMENTATION_DEFINED
        .iter()
        .find(|(listed, _)| *listed == name)
    {
        return Some(*verdict);
    }
    if DUPLICATE_KEYS.contains(&name) {
        Some(Verdict::Refused)
    } else if name.starts_with("y_") {
        Some(Verdict::Reads)
    } else if name.starts_with("n_") {
        Some(Verdict::Refused)
    } else {
        None
    }
}

/// Whether the file at `path` reads, under `limits`.
fn reads(path: &Path, limits: Limits) -> bool {
    Input::open(path)
        .and_then(|input| input.limits(limits).parse::<IgnoredAny>())
        .is_ok()
}

#[test]
fn every_file_of_the_suite_has_the_verdict_the_reader_gives() {
    let mut names = BTreeSet::new();
    let mut found = Vec::new();
    let mut wrong = Vec::new();
    let entries = fs::read_dir(suite()).unwrap_or_else(|error| panic!("{error}"));
    for entry in entries {
        let path = entry.unwrap_or_else(|error| panic!("{error}")).path();
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let Some(expected) = verdict(&name) else {
            wrong.push(format!("{name}: no verdict"));
            continue;
        };
        let by_default = reads(&path, Limits::NONE);
        match expected {
            Verdict::Reads if !by_default => wrong.push(format!("{name}: refused")),
            Verdict::Refused if by_default => wrong.push(format!("{name}: read")),
            Verdict::ReadsFromDepth(depth) => {
                let limits = Limits {
                    max_depth: Some(depth),
                    ..Limits::NONE
                };
                let shallower = Limits {
                    max_depth: Some(depth - 1),
                    ..Limits::NONE
                };
                if by_default || !reads(&path, limits) || reads(&path, shallower) {
                    wrong.push(format!("{name}: not read from exactly depth {depth}"));
                }
            }
            Verdict::Reads | Verdict::Refused => {}
        }
        found.push(expected);
        names.insert(name);
    }
    assert!(wrong.is_empty(), "{wrong:#?}");

    let rows = IMPLEMENTATION_DEFINED
        .iter()
        .map(|(name, _)| *name)
        .chain(DUPLICATE_KEYS);
    let unmatched: Vec<_> = rows.filter(|name| !names.contains(*name)).collect();
    assert!(unmatched.is_empty(), "rows with no file: {unmatched:?}");

    let count =
        |wanted: fn(&Verdict) -> bool| found.iter().filter(|verdict| wanted(verdict)).count();
    assert_eq!(found.len(), 318);
    assert_eq!(count(|verdict| *verdict == Verdict::Reads), 104);
    assert_eq!(count(|verdict| *verdict == Verdict::Refused), 213);
    assert_eq!(
        count(|verdict| matches!(verdict, Verdict::ReadsFromDepth(_))),
        1
    );
}

#[test]
fn a_strict_reading_refuses_each_duplicate_key_file_as_a_duplicate_key() {
    for name in DUPLICATE_KEYS {
        let path = suite().join(format!("{name}.json"));
        let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{error}"));
        let refusal = JSON
            .read(&text, &Limits::NONE, Reading::Strict)
            .unwrap_err();
        assert_eq!(
            refusal.fault(),
            &Fault::DuplicateKey {
                key: "a".to_owned()
            },
            "{name}"
        );
        let span = refusal.span().unwrap_or_else(|| panic!("{name}: no span"));
        assert_eq!(&text[span.clone()], "\"a\"", "{name}");
        assert_eq!(span.start, 9, "{name}");
    }
}

#[test]
fn a_listing_keeps_the_keys_a_strict_reading_refuses() {
    for name in DUPLICATE_KEYS {
        let input = Input::open(&suite().join(format!("{name}.json")))
            .unwrap_or_else(|error| panic!("{error}"));
        let entries = input.top_level().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(entries.len(), 2, "{name}");
        assert_eq!(entries[0].key, entries[1].key, "{name}");
    }
}

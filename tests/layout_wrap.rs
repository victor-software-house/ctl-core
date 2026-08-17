//! Terminal wrap / indent helpers.
#![allow(missing_docs)]
#![cfg(feature = "help")]

#[test]
fn push_line_terminates() {
    for text in ["hello", "Usage: toy <COMMAND>", "--dry-run", "x"] {
        let mut out = String::new();
        ctl_core::layout::push_line(&mut out, text);
        assert!(out.ends_with('\n'), "{out:?}");
        assert!(out.contains(text.trim()), "{out}");
    }
}

#[test]
fn indent_prefixes() {
    let mut out = String::new();
    ctl_core::layout::push_indented(&mut out, "flag", 2);
    assert!(out.starts_with("  flag"), "{out:?}");
}

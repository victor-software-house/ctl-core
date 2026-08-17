//! ANSI wrap / empty.
#![allow(missing_docs)]
#![cfg(feature = "color")]

#[test]
fn empty_stays_empty() {
    assert_eq!(ctl_core::style::styled(ctl_core::style::OPTION, ""), "");
}

#[test]
fn wraps_nonempty() {
    let out = ctl_core::style::styled(ctl_core::style::OPTION, "--help");
    assert!(out.contains("--help"));
    assert_ne!(out, "--help");
}

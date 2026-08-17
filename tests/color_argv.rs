//! `-c/--color` / `--no-color` last-wins and token parse.
#![allow(missing_docs)]

use ctl_core::color::ParseColorError;
use ctl_core::flags::resolve_color;
use ctl_core::prelude::*;

macro_rules! from_args {
    ($($name:ident: [$($arg:literal),*] => $want:ident,)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(ColorMode::from_args([$($arg),*]), ColorMode::$want);
            }
        )*
    };
}

from_args! {
    default_is_auto: ["bin"] => Auto,
    long_auto: ["bin", "--color", "auto"] => Auto,
    long_always: ["bin", "--color", "always"] => Always,
    long_never: ["bin", "--color", "never"] => Never,
    short_auto: ["bin", "-c", "auto"] => Auto,
    short_always: ["bin", "-c", "always"] => Always,
    short_never: ["bin", "-c", "never"] => Never,
    equals_auto: ["bin", "--color=auto"] => Auto,
    equals_always: ["bin", "--color=always"] => Always,
    equals_never: ["bin", "--color=never"] => Never,
    no_color: ["bin", "--no-color"] => Never,
    no_color_after_always: ["bin", "--color", "always", "--no-color"] => Never,
    always_after_no_color: ["bin", "--no-color", "--color", "always"] => Always,
    never_after_no_color: ["bin", "--no-color", "--color", "never"] => Never,
    auto_after_no_color: ["bin", "--no-color", "--color", "auto"] => Auto,
    no_color_after_equals: ["bin", "--color=always", "--no-color"] => Never,
    equals_after_no_color: ["bin", "--no-color", "--color=always"] => Always,
    short_after_no_color: ["bin", "--no-color", "-c", "always"] => Always,
    no_color_after_short: ["bin", "-c", "always", "--no-color"] => Never,
    last_of_three: ["bin", "--color", "never", "-c", "auto", "--color=always"] => Always,
    junk_value_keeps_default: ["bin", "--color", "rainbow"] => Auto,
    color_then_no_color_not_swallowed: ["bin", "--color", "--no-color"] => Never,
    unknown_flag_ignored: ["bin", "--format", "json"] => Auto,
    last_never_wins: ["bin", "--color", "always", "--color", "never"] => Never,
    last_always_wins: ["bin", "--color", "never", "--color", "always"] => Always,
    short_then_equals: ["bin", "-c", "never", "--color=always"] => Always,
    equals_then_short: ["bin", "--color=always", "-c", "never"] => Never,
    no_color_twice: ["bin", "--no-color", "--no-color"] => Never,
    no_color_always_no_color: ["bin", "--no-color", "--color", "always", "--no-color"] => Never,
    after_subcommand: ["bin", "status", "--color", "never"] => Never,
    help_flag_ignored: ["bin", "--help"] => Auto,
    version_flag_ignored: ["bin", "--version"] => Auto,
    empty_equals_ignored: ["bin", "--color="] => Auto,
}

#[test]
fn parse_known_tokens() {
    assert_eq!("auto".parse::<ColorMode>().unwrap(), ColorMode::Auto);
    assert_eq!("always".parse::<ColorMode>().unwrap(), ColorMode::Always);
    assert_eq!("never".parse::<ColorMode>().unwrap(), ColorMode::Never);
}

#[test]
fn parse_rejects_unknown_tokens() {
    for raw in ["", "AUTO", "true", "on", "off", "none", " auto"] {
        assert_eq!(raw.parse::<ColorMode>(), Err(ParseColorError), "{raw:?}");
    }
}

#[test]
fn display_roundtrip() {
    for mode in [ColorMode::Auto, ColorMode::Always, ColorMode::Never] {
        assert_eq!(mode.to_string().parse::<ColorMode>().unwrap(), mode);
    }
}

#[test]
fn no_color_flag_overrides_value() {
    assert_eq!(resolve_color(ColorMode::Always, true), ColorMode::Never);
    assert_eq!(resolve_color(ColorMode::Always, false), ColorMode::Always);
    assert_eq!(resolve_color(ColorMode::Auto, true), ColorMode::Never);
}

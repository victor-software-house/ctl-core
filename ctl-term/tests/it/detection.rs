//! `Terminal::from_probe` over every variable that decides colour, depth, and
//! width: each alone and in conflicting pairs.

use ctl_term::{ColorDepth, ColorMode, DetectOptions, Probe, Terminal};

use crate::common::{PIPE, TERMINAL, Variables, lookup, terminal};

/// One environment and the colour it gives under `Auto`, on a pipe and on a
/// terminal.
struct Case {
    name: &'static str,
    variables: Variables,
    on_pipe: bool,
    on_terminal: bool,
}

const fn case(name: &'static str, variables: Variables, on_pipe: bool, on_terminal: bool) -> Case {
    Case {
        name,
        variables,
        on_pipe,
        on_terminal,
    }
}

const COLOR_CASES: &[Case] = &[
    case("no variable", &[], false, true),
    case("NO_COLOR set", &[("NO_COLOR", "1")], false, false),
    case("NO_COLOR empty", &[("NO_COLOR", "")], false, true),
    case("CLICOLOR_FORCE=1", &[("CLICOLOR_FORCE", "1")], true, true),
    case("CLICOLOR_FORCE=0", &[("CLICOLOR_FORCE", "0")], false, true),
    case(
        "CLICOLOR_FORCE empty",
        &[("CLICOLOR_FORCE", "")],
        false,
        true,
    ),
    case("FORCE_COLOR=1", &[("FORCE_COLOR", "1")], true, true),
    case("FORCE_COLOR=3", &[("FORCE_COLOR", "3")], true, true),
    case("FORCE_COLOR=true", &[("FORCE_COLOR", "true")], true, true),
    case("FORCE_COLOR=0", &[("FORCE_COLOR", "0")], false, true),
    case(
        "FORCE_COLOR=false",
        &[("FORCE_COLOR", "false")],
        false,
        true,
    ),
    case("FORCE_COLOR empty", &[("FORCE_COLOR", "")], false, true),
    case("CLICOLOR=0", &[("CLICOLOR", "0")], false, false),
    case("CLICOLOR=1", &[("CLICOLOR", "1")], false, true),
    case("TERM=dumb", &[("TERM", "dumb")], false, false),
    case("TERM=xterm", &[("TERM", "xterm")], false, true),
    case(
        "NO_COLOR over CLICOLOR_FORCE",
        &[("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")],
        false,
        false,
    ),
    case(
        "NO_COLOR over FORCE_COLOR",
        &[("NO_COLOR", "1"), ("FORCE_COLOR", "1")],
        false,
        false,
    ),
    case(
        "NO_COLOR over both forces",
        &[
            ("NO_COLOR", "1"),
            ("CLICOLOR_FORCE", "1"),
            ("FORCE_COLOR", "3"),
        ],
        false,
        false,
    ),
    case(
        "NO_COLOR over CLICOLOR=1",
        &[("NO_COLOR", "1"), ("CLICOLOR", "1")],
        false,
        false,
    ),
    case(
        "empty NO_COLOR does not beat CLICOLOR_FORCE",
        &[("NO_COLOR", ""), ("CLICOLOR_FORCE", "1")],
        true,
        true,
    ),
    case(
        "CLICOLOR_FORCE=1 with FORCE_COLOR=0",
        &[("CLICOLOR_FORCE", "1"), ("FORCE_COLOR", "0")],
        true,
        true,
    ),
    case(
        "CLICOLOR_FORCE=0 with FORCE_COLOR=1",
        &[("CLICOLOR_FORCE", "0"), ("FORCE_COLOR", "1")],
        true,
        true,
    ),
    case(
        "CLICOLOR_FORCE=0 with FORCE_COLOR=0",
        &[("CLICOLOR_FORCE", "0"), ("FORCE_COLOR", "0")],
        false,
        true,
    ),
    case(
        "CLICOLOR_FORCE over CLICOLOR=0",
        &[("CLICOLOR_FORCE", "1"), ("CLICOLOR", "0")],
        true,
        true,
    ),
    case(
        "CLICOLOR_FORCE over TERM=dumb",
        &[("CLICOLOR_FORCE", "1"), ("TERM", "dumb")],
        true,
        true,
    ),
    case(
        "FORCE_COLOR over CLICOLOR=0",
        &[("FORCE_COLOR", "1"), ("CLICOLOR", "0")],
        true,
        true,
    ),
    case(
        "FORCE_COLOR over TERM=dumb",
        &[("FORCE_COLOR", "1"), ("TERM", "dumb")],
        true,
        true,
    ),
    case(
        "FORCE_COLOR=0 with CLICOLOR=0",
        &[("FORCE_COLOR", "0"), ("CLICOLOR", "0")],
        false,
        false,
    ),
    case(
        "FORCE_COLOR=0 with TERM=dumb",
        &[("FORCE_COLOR", "0"), ("TERM", "dumb")],
        false,
        false,
    ),
    case(
        "CLICOLOR=0 with TERM=dumb",
        &[("CLICOLOR", "0"), ("TERM", "dumb")],
        false,
        false,
    ),
    case(
        "CLICOLOR=1 does not beat TERM=dumb",
        &[("CLICOLOR", "1"), ("TERM", "dumb")],
        false,
        false,
    ),
];

#[test]
fn auto_colour_follows_the_fixed_order() {
    for case in COLOR_CASES {
        let pipe = terminal(PIPE, ColorMode::Auto, case.variables);
        assert_eq!(pipe.color().is_some(), case.on_pipe, "pipe: {}", case.name);
        assert!(!pipe.is_attached(), "pipe: {}", case.name);
        let tty = terminal(TERMINAL, ColorMode::Auto, case.variables);
        assert_eq!(
            tty.color().is_some(),
            case.on_terminal,
            "terminal: {}",
            case.name
        );
        assert!(tty.is_attached(), "terminal: {}", case.name);
    }
}

#[test]
fn an_explicit_mode_ignores_the_colour_variables() {
    for case in COLOR_CASES {
        for probe in [PIPE, TERMINAL] {
            let always = terminal(probe, ColorMode::Always, case.variables);
            assert!(always.color().is_some(), "always: {}", case.name);
            let never = terminal(probe, ColorMode::Never, case.variables);
            assert_eq!(never.color(), None, "never: {}", case.name);
        }
    }
}

#[test]
fn the_default_mode_is_auto() {
    assert_eq!(ColorMode::default(), ColorMode::Auto);
}

/// One environment and the depth it gives when colour is on.
struct DepthCase {
    name: &'static str,
    variables: Variables,
    depth: ColorDepth,
}

const DEPTH_CASES: &[DepthCase] = &[
    depth_case("no variable", &[], ColorDepth::Ansi16),
    depth_case(
        "COLORTERM=truecolor",
        &[("COLORTERM", "truecolor")],
        ColorDepth::TrueColor,
    ),
    depth_case(
        "COLORTERM=24bit",
        &[("COLORTERM", "24bit")],
        ColorDepth::TrueColor,
    ),
    depth_case("COLORTERM=yes", &[("COLORTERM", "yes")], ColorDepth::Ansi16),
    depth_case("COLORTERM empty", &[("COLORTERM", "")], ColorDepth::Ansi16),
    depth_case(
        "TERM=xterm-256color",
        &[("TERM", "xterm-256color")],
        ColorDepth::Ansi256,
    ),
    depth_case(
        "TERM=screen-256color",
        &[("TERM", "screen-256color")],
        ColorDepth::Ansi256,
    ),
    depth_case("TERM=xterm", &[("TERM", "xterm")], ColorDepth::Ansi16),
    depth_case("TERM=dumb", &[("TERM", "dumb")], ColorDepth::Ansi16),
    depth_case(
        "FORCE_COLOR=3",
        &[("FORCE_COLOR", "3")],
        ColorDepth::TrueColor,
    ),
    depth_case(
        "FORCE_COLOR=2",
        &[("FORCE_COLOR", "2")],
        ColorDepth::Ansi256,
    ),
    depth_case("FORCE_COLOR=1", &[("FORCE_COLOR", "1")], ColorDepth::Ansi16),
    depth_case("FORCE_COLOR=0", &[("FORCE_COLOR", "0")], ColorDepth::Ansi16),
    depth_case(
        "COLORTERM beats TERM",
        &[("COLORTERM", "truecolor"), ("TERM", "xterm-256color")],
        ColorDepth::TrueColor,
    ),
    depth_case(
        "FORCE_COLOR=3 beats TERM",
        &[("FORCE_COLOR", "3"), ("TERM", "xterm-256color")],
        ColorDepth::TrueColor,
    ),
    depth_case(
        "COLORTERM beats FORCE_COLOR=2",
        &[("COLORTERM", "truecolor"), ("FORCE_COLOR", "2")],
        ColorDepth::TrueColor,
    ),
    depth_case(
        "FORCE_COLOR=2 beats TERM=xterm",
        &[("FORCE_COLOR", "2"), ("TERM", "xterm")],
        ColorDepth::Ansi256,
    ),
    depth_case(
        "FORCE_COLOR=1 keeps TERM=xterm-256color",
        &[("FORCE_COLOR", "1"), ("TERM", "xterm-256color")],
        ColorDepth::Ansi256,
    ),
];

const fn depth_case(name: &'static str, variables: Variables, depth: ColorDepth) -> DepthCase {
    DepthCase {
        name,
        variables,
        depth,
    }
}

#[test]
fn depth_follows_colorterm_term_and_force_color() {
    for case in DEPTH_CASES {
        for probe in [PIPE, TERMINAL] {
            let always = terminal(probe, ColorMode::Always, case.variables);
            assert_eq!(always.color(), Some(case.depth), "always: {}", case.name);
        }
    }
}

#[test]
fn a_forced_colour_takes_the_depth_the_variables_give() {
    let variables: Variables = &[("FORCE_COLOR", "3")];
    let terminal = terminal(PIPE, ColorMode::Auto, variables);
    assert_eq!(terminal.color(), Some(ColorDepth::TrueColor));
}

#[test]
fn with_depth_replaces_the_depth_of_a_coloured_terminal() {
    let coloured = terminal(PIPE, ColorMode::Always, &[("COLORTERM", "truecolor")]);
    for depth in [
        ColorDepth::Ansi16,
        ColorDepth::Ansi256,
        ColorDepth::TrueColor,
    ] {
        assert_eq!(coloured.with_depth(depth).color(), Some(depth));
    }
}

#[test]
fn with_depth_does_not_turn_colour_on() {
    let plain = terminal(PIPE, ColorMode::Never, &[]);
    assert_eq!(plain.with_depth(ColorDepth::TrueColor).color(), None);
}

#[test]
fn depths_are_ordered_from_fewest_colours_to_most() {
    assert!(ColorDepth::Ansi16 < ColorDepth::Ansi256);
    assert!(ColorDepth::Ansi256 < ColorDepth::TrueColor);
}

/// One probe, environment, and options, and the width they give.
struct WidthCase {
    name: &'static str,
    probe: Probe,
    variables: Variables,
    options: DetectOptions,
    width: Option<u16>,
}

const fn width_case(
    name: &'static str,
    probe: Probe,
    variables: Variables,
    options: DetectOptions,
    width: Option<u16>,
) -> WidthCase {
    WidthCase {
        name,
        probe,
        variables,
        options,
        width,
    }
}

const fn window(columns: u16) -> Probe {
    Probe::terminal(Some(columns))
}

const UNSIZED_WINDOW: Probe = Probe::terminal(None);

/// The buffer variable ctl-core names, which `DetectOptions::new` does not
/// read.
const BUFFER_ENVS: &[&str] = &["CTL_CORE_COLUMN_BUFFER"];

const COLUMNS_100: Variables = &[("COLUMNS", "100")];

fn source_cases() -> Vec<WidthCase> {
    let defaults = DetectOptions::new();
    vec![
        width_case(
            "a pipe with nothing to detect",
            PIPE,
            &[],
            defaults,
            Some(80),
        ),
        width_case("COLUMNS on a pipe", PIPE, COLUMNS_100, defaults, Some(98)),
        width_case("COLUMNS=0", PIPE, &[("COLUMNS", "0")], defaults, Some(80)),
        width_case(
            "COLUMNS not a number",
            PIPE,
            &[("COLUMNS", "wide")],
            defaults,
            Some(80),
        ),
        width_case(
            "COLUMNS negative",
            PIPE,
            &[("COLUMNS", "-5")],
            defaults,
            Some(80),
        ),
        width_case(
            "COLUMNS empty",
            PIPE,
            &[("COLUMNS", "")],
            defaults,
            Some(80),
        ),
        width_case(
            "COLUMNS above u16",
            PIPE,
            &[("COLUMNS", "70000")],
            defaults,
            Some(80),
        ),
        width_case(
            "COLUMNS under the minimum",
            PIPE,
            &[("COLUMNS", "21")],
            defaults,
            Some(20),
        ),
        width_case(
            "COLUMNS narrower than the minimum",
            PIPE,
            &[("COLUMNS", "15")],
            defaults,
            Some(15),
        ),
        width_case(
            "the window beats COLUMNS",
            window(120),
            &[("COLUMNS", "60")],
            defaults,
            Some(118),
        ),
        width_case(
            "a window with no size uses COLUMNS",
            UNSIZED_WINDOW,
            &[("COLUMNS", "60")],
            defaults,
            Some(58),
        ),
        width_case(
            "a window with no size and no COLUMNS",
            UNSIZED_WINDOW,
            &[],
            defaults,
            Some(80),
        ),
        width_case(
            "an explicit width is exact",
            window(120),
            COLUMNS_100,
            defaults.width(50),
            Some(50),
        ),
        width_case(
            "an explicit width on a pipe",
            PIPE,
            &[],
            defaults.width(50),
            Some(50),
        ),
        width_case(
            "an explicit width under the minimum",
            window(120),
            &[],
            defaults.width(5),
            Some(5),
        ),
        width_case(
            "an explicit width of 0 counts as 1",
            window(120),
            &[],
            defaults.width(0),
            Some(1),
        ),
        width_case(
            "a window of 0 columns is not reported",
            window(0),
            &[("COLUMNS", "60")],
            defaults,
            Some(58),
        ),
    ]
}

fn buffer_cases() -> Vec<WidthCase> {
    let defaults = DetectOptions::new();
    vec![
        width_case(
            "no buffer",
            PIPE,
            COLUMNS_100,
            defaults.column_buffer(0),
            Some(100),
        ),
        width_case(
            "a buffer of 5",
            PIPE,
            COLUMNS_100,
            defaults.column_buffer(5),
            Some(95),
        ),
        width_case(
            "the buffer variable",
            PIPE,
            &[("COLUMNS", "100"), ("CTL_CORE_COLUMN_BUFFER", "4")],
            defaults.column_buffer_envs(BUFFER_ENVS),
            Some(96),
        ),
        width_case(
            "an explicit buffer beats the variable",
            PIPE,
            &[("COLUMNS", "100"), ("CTL_CORE_COLUMN_BUFFER", "4")],
            defaults.column_buffer_envs(BUFFER_ENVS).column_buffer(1),
            Some(99),
        ),
        width_case(
            "the buffer variable is not a number",
            PIPE,
            &[("COLUMNS", "100"), ("CTL_CORE_COLUMN_BUFFER", "many")],
            defaults.column_buffer_envs(BUFFER_ENVS),
            Some(98),
        ),
        width_case(
            "replaced buffer names, first number wins",
            PIPE,
            &[
                ("COLUMNS", "100"),
                ("FIRST", "x"),
                ("SECOND", "3"),
                ("THIRD", "9"),
            ],
            defaults.column_buffer_envs(&["FIRST", "SECOND", "THIRD"]),
            Some(97),
        ),
        width_case(
            "buffer names do not read a variable they leave out",
            PIPE,
            &[("COLUMNS", "100"), ("CTL_CORE_COLUMN_BUFFER", "4")],
            defaults.column_buffer_envs(&["OTHER"]),
            Some(98),
        ),
        width_case(
            "the defaults read no buffer variable",
            PIPE,
            &[("COLUMNS", "100"), ("CTL_CORE_COLUMN_BUFFER", "4")],
            defaults,
            Some(98),
        ),
        width_case(
            "an empty buffer name list reads no variable",
            PIPE,
            &[("COLUMNS", "100"), ("CTL_CORE_COLUMN_BUFFER", "4")],
            defaults
                .column_buffer_envs(BUFFER_ENVS)
                .column_buffer_envs(&[]),
            Some(98),
        ),
    ]
}

fn limit_cases() -> Vec<WidthCase> {
    let defaults = DetectOptions::new();
    vec![
        width_case(
            "no minimum",
            PIPE,
            &[("COLUMNS", "3")],
            defaults.minimum_width(0),
            Some(1),
        ),
        width_case(
            "a higher minimum",
            PIPE,
            COLUMNS_100,
            defaults.minimum_width(99),
            Some(99),
        ),
        width_case(
            "a minimum below the buffered width",
            PIPE,
            COLUMNS_100,
            defaults.minimum_width(30),
            Some(98),
        ),
        width_case(
            "another fallback",
            PIPE,
            &[],
            defaults.fallback_width(Some(60)),
            Some(60),
        ),
        width_case(
            "a fallback of 0 counts as 1",
            PIPE,
            &[],
            defaults.fallback_width(Some(0)),
            Some(1),
        ),
        width_case(
            "no fallback",
            PIPE,
            &[],
            defaults.fallback_width(None),
            None,
        ),
        width_case(
            "no fallback, COLUMNS set",
            PIPE,
            COLUMNS_100,
            defaults.fallback_width(None),
            Some(98),
        ),
        width_case(
            "the fallback is not buffered",
            PIPE,
            &[],
            defaults.fallback_width(Some(10)).column_buffer(5),
            Some(10),
        ),
    ]
}

fn width_cases() -> Vec<WidthCase> {
    [source_cases(), buffer_cases(), limit_cases()]
        .into_iter()
        .flatten()
        .collect()
}

#[test]
fn width_follows_the_stream_then_columns_then_the_fallback() {
    for case in width_cases() {
        for mode in [ColorMode::Auto, ColorMode::Always, ColorMode::Never] {
            let terminal =
                Terminal::from_probe(case.probe, mode, case.options, &lookup(case.variables));
            assert_eq!(terminal.width(), case.width, "{} under {mode:?}", case.name);
        }
    }
}

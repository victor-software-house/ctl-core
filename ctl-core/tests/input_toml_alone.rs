//! Declared input from TOML in a build with no YAML and no garde, as a
//! consumer that enables only `input-toml` gets it. `test:input-toml` runs
//! this file with that feature alone; a build with `input-yaml` skips it,
//! because its limit messages name the repository file.
//!
//! The process tier is global, so every test that sets it runs in the one
//! sequence test, and the other tests set every field they depend on through
//! the call tier.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "input-toml", not(feature = "input-yaml")))]

use std::path::Path;

use ctl_core::indoc;
use ctl_core::input::{FieldPath, Format, Input, Limits, Problem, set_default_limits};
use serde::Deserialize;
use serde::de::IgnoredAny;

fn toml(name: &str, text: &str) -> Input {
    Input::new_with_format(name, text, Format::Toml)
}

/// `depth` inline tables nested in the root table, so the innermost one sits
/// at depth `depth + 1`.
fn nested(depth: usize) -> String {
    let mut value = "1".to_owned();
    for _ in 0..depth {
        value = format!("{{ a = {value} }}");
    }
    format!("a = {value}\n")
}

fn depth(limit: usize) -> Limits {
    Limits {
        max_depth: Some(limit),
        ..Limits::NONE
    }
}

/// The message of the one problem a parse reports, or `None` if it parsed.
fn refusal(input: &Input) -> Option<String> {
    let error = input.parse::<IgnoredAny>().err()?;
    assert_eq!(error.problems.len(), 1, "{error}");
    Some(error.problems[0].message.clone())
}

#[test]
fn text_in_memory_parses_without_yaml() {
    #[derive(Debug, Deserialize)]
    struct Config {
        name: String,
    }
    let config: Config = toml("app.toml", "name = 'release'\n").parse().unwrap();
    assert_eq!(config.name, "release");
}

#[test]
fn limits_build_from_none_whatever_the_build_enables() {
    let limits = Limits {
        max_reader_input_bytes: Some(1),
        max_depth: Some(2),
        ..Limits::NONE
    };
    assert_eq!(limits.max_reader_input_bytes, Some(1));
    assert_eq!(limits.max_nodes, None);
    assert_eq!(limits.clone().or(&Limits::NONE), limits);
}

#[test]
fn the_byte_limit_says_the_tool_sets_it() {
    let input = toml("big.toml", "name = 'release'\n").limits(Limits {
        max_reader_input_bytes: Some(10),
        ..Limits::NONE
    });
    let error = input.parse::<IgnoredAny>().unwrap_err();
    assert_eq!(
        error.problems,
        [Problem {
            line: None,
            column: None,
            field: String::new(),
            message: "is 17 bytes, which exceeds max_reader_input_bytes (10); \
                      the tool sets this limit"
                .to_owned(),
        }]
    );
}

#[test]
fn the_depth_limit_says_the_tool_sets_it() {
    let input = toml(
        "deep.toml",
        indoc! {"
            a = { b = { c = { d = { e = 1 } } } }
        "},
    );
    let error = input
        .clone()
        .limits(depth(4))
        .parse::<IgnoredAny>()
        .unwrap_err();
    assert_eq!(
        error.problems,
        [Problem {
            line: Some(1),
            column: Some(23),
            field: String::new(),
            message: "exceeds max_depth (4); the tool sets this limit".to_owned(),
        }]
    );
    assert!(input.limits(depth(5)).parse::<IgnoredAny>().is_ok());
}

#[test]
fn the_node_limit_says_the_tool_sets_it() {
    let items = vec!["1"; 30].join(", ");
    let wide = toml("wide.toml", &format!("values = [{items}]\n"));
    let error = wide
        .clone()
        .limits(Limits {
            max_nodes: Some(20),
            ..Limits::NONE
        })
        .parse::<IgnoredAny>()
        .unwrap_err();
    assert_eq!(
        error.problems,
        [Problem {
            line: Some(1),
            column: Some(65),
            field: String::new(),
            message: "exceeds max_nodes (20); the tool sets this limit".to_owned(),
        }],
        "the root, the array, and 18 items make 20; the 19th item crosses"
    );
    let roomy = Limits {
        max_nodes: Some(32),
        ..Limits::NONE
    };
    assert!(wide.limits(roomy).parse::<IgnoredAny>().is_ok());
}

#[test]
fn a_yaml_file_names_the_feature_it_needs() {
    for name in ["app.yaml", "app.yml"] {
        let error = Input::open(Path::new(name)).unwrap_err();
        let extension = name.trim_start_matches("app");
        assert_eq!(
            error.problems[0].message,
            format!("`{extension}` needs ctl-core's input-yaml feature; name the file *.toml"),
        );
        assert_eq!(error.problems[0].line, None);
    }
    let error = Input::open(Path::new("app.json")).unwrap_err();
    assert_eq!(
        error.problems[0].message,
        "unsupported extension .json; name the file *.toml"
    );
}

/// The default, then the process, then the call. Every test that reads the
/// default or sets the process tier runs here, in order.
#[test]
fn the_default_then_the_process_then_the_call_decide_each_limit() {
    assert_eq!(
        refusal(&toml("deep.toml", &nested(64))).as_deref(),
        Some("exceeds max_depth (64); the tool sets this limit"),
        "ctl-core's default depth holds a TOML text"
    );
    assert_eq!(refusal(&toml("deep.toml", &nested(63))), None);

    set_default_limits(depth(4));
    let six = toml("six.toml", &nested(5));
    assert_eq!(
        refusal(&six).as_deref(),
        Some("exceeds max_depth (4); the tool sets this limit")
    );
    assert_eq!(
        refusal(&six.clone().limits(depth(5))).as_deref(),
        Some("exceeds max_depth (5); the tool sets this limit"),
        "the call beats the process"
    );
    assert_eq!(refusal(&six.limits(depth(6))), None);
    set_default_limits(Limits::NONE);
}

#[test]
fn a_path_is_placed_without_yaml_and_without_garde() {
    let input = toml("a.toml", "[files.\"docs/**/*.md\"]\nlabel = ''\n");
    let path = FieldPath::root()
        .key("files")
        .key("docs/**/*.md")
        .key("label");
    let at = input.place(&path).map(|at| (at.line, at.column));
    assert_eq!(at, Some((2, 9)));
    let problem = input.problem_at(&path, "is not a known label");
    assert_eq!(problem.field, "files.docs/**/*.md.label");
    assert_eq!((problem.line, problem.column), (Some(2), Some(9)));
}

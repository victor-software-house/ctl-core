//! Input limits: serde-saphyr's defaults unless overridden, field by field, by
//! the call, then the repository, then the process.
//!
//! The repository and process tiers are global, so every test that sets one
//! runs in this binary's single sequence test, and the other tests set every
//! field they depend on through the call tier.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "input", feature = "fixture"))]

use ctl_core::fixture::Suite;
use ctl_core::indoc;
use ctl_core::input::{Input, Limits, REPO_FILE, set_default_limits, use_repo_limits};
use serde::de::IgnoredAny;

/// Mappings nested `depth` deep, one key per level.
fn nested(depth: usize) -> String {
    (0..depth)
        .map(|level| format!("{}a:", "  ".repeat(level)))
        .collect::<Vec<_>>()
        .join("\n")
        + " 1\n"
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
fn a_limit_error_names_the_limit_the_file_and_where_to_raise_it() {
    let input = Input::new("deep.yaml", nested(6)).limits(depth(4));
    let error = input.parse::<IgnoredAny>().unwrap_err();
    let shown = error.to_string();
    assert!(shown.starts_with("deep.yaml:"), "{shown}");
    assert!(
        shown.ends_with("exceeds max_depth (4); raise limits.max_depth in .ctl/input.yaml"),
        "{shown}"
    );
    assert!(error.problems[0].line.is_some(), "{shown}");
}

#[test]
fn text_already_in_memory_is_held_to_the_input_byte_limit() {
    let limits = Limits {
        max_reader_input_bytes: Some(10),
        ..Limits::NONE
    };
    let input = Input::new("big.yaml", "a: 12345678901234\n").limits(limits);
    let error = input.parse::<IgnoredAny>().unwrap_err();
    assert_eq!(error.problems[0].line, None, "{error}");
    assert_eq!(
        error.problems[0].message,
        "is 18 bytes, which exceeds max_reader_input_bytes (10); \
         raise limits.max_reader_input_bytes in .ctl/input.yaml"
    );
}

#[test]
fn the_call_then_the_repository_then_the_process_decide_each_field() {
    let suite = Suite::new("input-limits");
    let six = Input::new("six.yaml", nested(6));

    assert_eq!(refusal(&six), None, "serde-saphyr's depth 64 admits six");

    set_default_limits(depth(4));
    assert_eq!(
        refusal(&six).as_deref(),
        Some("exceeds max_depth (4); raise limits.max_depth in .ctl/input.yaml"),
        "the process default applies"
    );

    let repo = suite.scenario("raised");
    repo.file(
        REPO_FILE,
        indoc! {"
            limits:
              max_depth: 8
        "},
    );
    use_repo_limits(repo.path()).unwrap();
    assert_eq!(refusal(&six), None, "the repository beats the process");

    let five = Input::new("six.yaml", nested(6)).limits(depth(5));
    assert!(
        refusal(&five).is_some_and(|message| message.contains("max_depth (5)")),
        "the call beats the repository"
    );

    set_default_limits(Limits {
        max_depth: Some(4),
        max_nodes: Some(20),
        ..Limits::NONE
    });
    let wide = Input::new("wide.yaml", format!("[{}]\n", vec!["1"; 30].join(", ")));
    assert!(
        refusal(&wide).is_some_and(|message| message.contains("max_nodes (20)")),
        "a field the repository leaves unset falls through to the process"
    );
    assert_eq!(
        refusal(&six),
        None,
        "max_depth still comes from the repository"
    );

    let empty = suite.scenario("no-file");
    use_repo_limits(empty.path()).unwrap();
    assert!(
        refusal(&six).is_some_and(|message| message.contains("max_depth (4)")),
        "a repository without the file overrides nothing"
    );

    let wrong = suite.scenario("unknown");
    wrong.file(
        REPO_FILE,
        indoc! {"
            limits:
              max_dept: 8
        "},
    );
    let error = use_repo_limits(wrong.path()).unwrap_err().to_string();
    assert!(error.contains("input.yaml"), "{error}");
    assert!(error.contains("max_dept"), "{error}");

    set_default_limits(Limits::NONE);
    let tight = suite.scenario("tight");
    tight.file(
        REPO_FILE,
        indoc! {"
            limits:
              max_depth: 1
        "},
    );
    use_repo_limits(tight.path()).unwrap();
    use_repo_limits(repo.path())
        .expect("a repository file is read under the process default, not the last repository");
    assert_eq!(
        refusal(&six),
        None,
        "the second repository replaced the first"
    );
    use_repo_limits(empty.path()).unwrap();

    set_default_limits(depth(1));
    let error = use_repo_limits(tight.path()).unwrap_err().to_string();
    assert!(
        error.ends_with(
            "exceeds max_depth (1); raise max_depth in the tool's process default, \
             which reads .ctl/input.yaml"
        ),
        "a limit hit reading the repository file points at the process default: {error}"
    );
    set_default_limits(Limits::NONE);
}

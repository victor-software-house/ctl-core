//! Declared input from TOML: parse, validate once, and place every problem on
//! its line and column, as `tests/input.rs` does for YAML.
//!
//! Limits are set through the call tier only: the repository and process
//! tiers are global, and `tests/input_limits.rs` owns them.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "input-toml", feature = "fixture"))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ctl_core::fixture::Suite;
use ctl_core::input::{Format, Input, Limits, Problem};
use ctl_core::validate::{at_least_one, cannot_be_empty, inside_the_repo, one_file_name};
use ctl_core::{formatdoc, indoc};
use garde::Validate;
use serde::Deserialize;
use serde::de::IgnoredAny;

#[derive(Debug, Deserialize, Validate, PartialEq)]
struct Config {
    #[garde(custom(cannot_be_empty("it names the release")))]
    name: String,
    #[garde(custom(at_least_one("label")))]
    labels: Vec<String>,
    #[garde(dive)]
    release: Release,
    #[garde(dive)]
    owner: Owner,
    #[garde(dive)]
    #[serde(default)]
    outputs: Vec<Output>,
}

#[derive(Debug, Deserialize, Validate, PartialEq)]
struct Release {
    #[garde(custom(cannot_be_empty("it names the tag")))]
    tag: String,
}

#[derive(Debug, Deserialize, Validate, PartialEq)]
struct Owner {
    #[garde(custom(cannot_be_empty("it names who answers")))]
    name: String,
}

#[derive(Debug, Deserialize, Validate, PartialEq)]
struct Output {
    #[garde(custom(inside_the_repo))]
    path: PathBuf,
    #[garde(custom(one_file_name))]
    #[serde(default)]
    file: String,
}

fn problem(line: usize, column: usize, field: &str, message: &str) -> Problem {
    Problem {
        line: Some(line),
        column: Some(column),
        field: field.to_owned(),
        message: message.to_owned(),
    }
}

#[test]
fn a_valid_file_loads_through_its_extension() {
    let suite = Suite::new("input-toml");
    let root = suite.scenario("valid");
    let path = root.file(
        "app.toml",
        indoc! {"
            name = 'release'
            labels = ['ship']
            owner = { name = 'victor' }

            [release]
            tag = 'v1'

            [[outputs]]
            path = 'docs'
            file = 'a.md'
        "},
    );
    let config: Config = Input::open(&path).unwrap().load().unwrap();
    assert_eq!(config.name, "release");
    assert_eq!(config.labels, ["ship"]);
    assert_eq!(config.owner.name, "victor");
    assert_eq!(config.release.tag, "v1");
    assert_eq!(config.outputs[0].file, "a.md");
}

#[test]
fn every_schema_problem_is_reported_on_its_line_and_column() {
    let suite = Suite::new("input-toml");
    let root = suite.scenario("problems");
    let path = root.file(
        "app.toml",
        indoc! {"
            name = ''
            labels = []
            owner = { name = '' }

            [release]
            tag = ''

            [[outputs]]
            path = 'docs'
            file = 'a.md'

            [[outputs]]
            path = '../outside'
            file = 'nested/b.md'
        "},
    );
    let error = Input::open(&path).unwrap().load::<Config>().unwrap_err();
    let expected = vec![
        problem(1, 8, "name", "cannot be empty — it names the release"),
        problem(2, 10, "labels", "must declare at least one label"),
        problem(
            3,
            18,
            "owner.name",
            "cannot be empty — it names who answers",
        ),
        problem(6, 7, "release.tag", "cannot be empty — it names the tag"),
        problem(13, 8, "outputs[1].path", "must stay inside the repository"),
        problem(14, 8, "outputs[1].file", "must be one file name"),
    ];
    let mut problems = error.problems.clone();
    problems.sort_by_key(|problem| problem.line);
    assert_eq!(problems, expected);
    let shown = error.to_string();
    let named = format!(
        "{}:13:8: outputs[1].path: must stay inside the repository",
        path.display()
    );
    assert!(shown.contains(&named), "{shown}");
}

#[test]
fn a_dotted_key_is_placed_on_its_value() {
    let input = Input::new(
        "app.toml",
        indoc! {"
            name = 'release'
            labels = ['ship']
            owner.name = ''
            release.tag = 'v1'
        "},
    )
    .format(Format::Toml);
    let error = input.load::<Config>().unwrap_err();
    assert_eq!(
        error.problems,
        [problem(
            3,
            14,
            "owner.name",
            "cannot be empty — it names who answers"
        )]
    );
}

#[test]
fn a_field_the_file_leaves_out_sits_on_its_nearest_written_table() {
    let input = Input::new(
        "app.toml",
        indoc! {"
            name = 'release'
            labels = ['ship']
            owner = { name = 'victor' }
            release = { tag = 'v1' }

            [[outputs]]
            path = 'docs'
        "},
    )
    .format(Format::Toml);
    let error = input.load::<Config>().unwrap_err();
    assert_eq!(
        error.problems,
        [problem(6, 1, "outputs[0].file", "must be one file name")]
    );
}

#[test]
fn a_parse_error_names_its_line() {
    let input = Input::new(
        "app.toml",
        indoc! {"
            name = 'release'
            labels = ['ship'
        "},
    )
    .format(Format::Toml);
    let error = input.parse::<Config>().unwrap_err();
    assert_eq!(error.problems.len(), 1, "{error}");
    assert_eq!(error.problems[0].line, Some(2), "{error}");
    assert_eq!(error.problems[0].field, "");
    assert!(!error.problems[0].message.is_empty(), "{error}");
}

#[test]
fn a_type_error_and_a_missing_field_name_their_values() {
    type Ports = BTreeMap<String, u16>;
    let wrong = Input::new(
        "a.toml",
        indoc! {"
            port = 'eighty'
        "},
    )
    .format(Format::Toml);
    let error = wrong.parse::<Ports>().unwrap_err();
    assert_eq!(error.problems.len(), 1, "{error}");
    assert_eq!(error.problems[0].line, Some(1), "{error}");
    assert_eq!(error.problems[0].column, Some(8), "{error}");
    assert!(
        error.problems[0]
            .message
            .starts_with("invalid type: string")
            && error.problems[0].message.ends_with("expected u16"),
        "{error}"
    );
    let missing = Input::new(
        "b.toml",
        indoc! {"
            labels = ['ship']
        "},
    )
    .format(Format::Toml);
    let error = missing.parse::<Config>().unwrap_err();
    assert_eq!(
        error.problems,
        [problem(1, 1, "", "missing field `name`")],
        "a missing field sits on the enclosing table, here the file's start"
    );
}

#[test]
fn a_duplicate_key_is_refused_on_its_line() {
    type Flags = BTreeMap<String, bool>;
    let duplicate = Input::new(
        "c.toml",
        indoc! {"
            flag = true
            flag = false
        "},
    )
    .format(Format::Toml);
    let error = duplicate.parse::<Flags>().unwrap_err();
    assert_eq!(error.problems.len(), 1, "{error}");
    assert_eq!(error.problems[0].line, Some(2), "{error}");
    assert_eq!(error.problems[0].column, Some(1), "{error}");
}

#[test]
fn the_format_is_chosen_never_guessed() {
    let suite = Suite::new("input-toml");
    let root = suite.scenario("formats");
    let toml = "name = 'release'\n";
    for (name, format) in [
        ("a.toml", Format::Toml),
        ("b.TOML", Format::Toml),
        ("c.yaml", Format::Yaml),
        ("d.yml", Format::Yaml),
    ] {
        let path = root.file(name, toml);
        assert_eq!(Format::of(&path).unwrap(), format, "{name}");
        assert!(Input::open(&path).is_ok(), "{name}");
    }

    let json = root.file("app.json", "{}");
    let error = Input::open(&json).unwrap_err();
    assert_eq!(error.problems.len(), 1, "{error}");
    assert_eq!(error.problems[0].line, None, "{error}");
    assert_eq!(
        error.problems[0].message,
        "unsupported extension .json; name the file *.toml, *.yaml, or *.yml"
    );
    assert!(error.to_string().starts_with(&json.display().to_string()));

    let bare = root.file("app", "{}");
    assert_eq!(
        Input::open(&bare).unwrap_err().problems[0].message,
        "no file extension; name the file *.toml, *.yaml, or *.yml"
    );

    assert!(
        Input::open(Path::new("missing.json")).is_err(),
        "the extension is refused before the file is read"
    );
}

#[test]
fn a_name_never_changes_the_format_of_text_in_memory() {
    type Names = BTreeMap<String, String>;
    let text = indoc! {"
        name = 'release'
    "};
    let by_name = Input::new("app.toml", text);
    assert!(by_name.parse::<Names>().is_err(), "new is YAML");
    let named = Input::new("app.yaml", text).format(Format::Toml);
    assert_eq!(named.parse::<Names>().unwrap()["name"], "release");

    let suite = Suite::new("input-toml");
    let root = suite.scenario("read");
    let path = root.file("app.toml", text);
    assert!(
        Input::read(&path).unwrap().parse::<Names>().is_err(),
        "read is YAML whatever the extension"
    );
}

#[test]
fn frontmatter_is_yaml_whatever_the_inputs_format() {
    let file = Input::new(
        "a.md",
        indoc! {"
            ---
            name: release
            ---

            A body.
        "},
    )
    .format(Format::Toml);
    let (matter, _) = file.frontmatter().unwrap();
    let names: BTreeMap<String, String> = matter.parse().unwrap();
    assert_eq!(names["name"], "release");
}

#[test]
fn text_is_held_to_the_input_byte_limit() {
    let limits = Limits {
        max_reader_input_bytes: Some(10),
        ..Limits::NONE
    };
    let input = Input::new("big.toml", "name = 'release'\n")
        .format(Format::Toml)
        .limits(limits);
    let error = input.parse::<IgnoredAny>().unwrap_err();
    assert_eq!(error.problems[0].line, None, "{error}");
    assert_eq!(
        error.problems[0].message,
        "is 17 bytes, which exceeds max_reader_input_bytes (10); \
         raise limits.max_reader_input_bytes in .ctl/input.yaml"
    );
}

#[test]
fn nesting_is_held_to_the_depth_limit_at_the_value_that_crosses_it() {
    let nested = Input::new(
        "deep.toml",
        indoc! {"
            a = { b = { c = { d = { e = 1 } } } }
        "},
    )
    .format(Format::Toml);
    let four = Limits {
        max_depth: Some(4),
        ..Limits::NONE
    };
    let error = nested
        .clone()
        .limits(four)
        .parse::<IgnoredAny>()
        .unwrap_err();
    assert_eq!(
        error.problems,
        [problem(
            1,
            23,
            "",
            "exceeds max_depth (4); raise limits.max_depth in .ctl/input.yaml"
        )]
    );
    let five = Limits {
        max_depth: Some(5),
        ..Limits::NONE
    };
    assert!(nested.limits(five).parse::<IgnoredAny>().is_ok());
}

#[test]
fn an_array_of_tables_counts_as_two_levels() {
    let input = Input::new(
        "deep.toml",
        indoc! {"
            [[a]]
            b = 1
        "},
    )
    .format(Format::Toml);
    let two = Limits {
        max_depth: Some(2),
        ..Limits::NONE
    };
    let error = input.clone().limits(two).parse::<IgnoredAny>().unwrap_err();
    assert_eq!(error.problems[0].line, Some(1), "{error}");
    assert!(
        error.problems[0].message.contains("max_depth (2)"),
        "{error}"
    );
    let three = Limits {
        max_depth: Some(3),
        ..Limits::NONE
    };
    assert!(input.limits(three).parse::<IgnoredAny>().is_ok());
}

#[test]
fn the_node_limit_names_the_value_that_crosses_it() {
    let items = vec!["1"; 30].join(", ");
    let wide = Input::new(
        "wide.toml",
        formatdoc! {"
            values = [{items}]
        "},
    )
    .format(Format::Toml);
    let twenty = Limits {
        max_nodes: Some(20),
        ..Limits::NONE
    };
    let error = wide
        .clone()
        .limits(twenty)
        .parse::<IgnoredAny>()
        .unwrap_err();
    assert_eq!(
        error.problems,
        [problem(
            1,
            65,
            "",
            "exceeds max_nodes (20); raise limits.max_nodes in .ctl/input.yaml"
        )],
        "the root, the array, and 18 items make 20; the 19th item crosses"
    );
    let roomy = Limits {
        max_nodes: Some(32),
        ..Limits::NONE
    };
    assert!(wide.limits(roomy).parse::<IgnoredAny>().is_ok());
}

#[test]
fn a_yaml_only_limit_has_no_effect_on_toml() {
    let input = Input::new(
        "a.toml",
        indoc! {"
            name = 'release'
        "},
    )
    .format(Format::Toml)
    .limits(Limits {
        max_aliases: Some(0),
        max_documents: Some(0),
        ..Limits::NONE
    });
    assert!(input.parse::<IgnoredAny>().is_ok());
}

#[test]
fn nesting_past_the_parsers_own_bound_is_refused_on_a_line() {
    let open = "[".repeat(100);
    let close = "]".repeat(100);
    let input = Input::new(
        "deep.toml",
        formatdoc! {"
            a = {open}1{close}
        "},
    )
    .format(Format::Toml)
    .limits(Limits {
        max_depth: Some(1000),
        ..Limits::NONE
    });
    let error = input.parse::<IgnoredAny>().unwrap_err();
    assert_eq!(error.problems.len(), 1, "{error}");
    assert_eq!(error.problems[0].line, Some(1), "{error}");
}

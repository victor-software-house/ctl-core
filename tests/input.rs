//! Declared input: parse, validate once, and place every problem.
#![allow(missing_docs)]
#![cfg(feature = "input")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use ctl_core::indoc;
use ctl_core::input::{Input, Problem};
use ctl_core::validate::{at_least_one, cannot_be_empty, inside_the_repo, one_file_name};
use garde::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate, PartialEq)]
struct Config {
    #[garde(custom(cannot_be_empty("it names the release")))]
    name: String,
    #[garde(custom(at_least_one("label")))]
    labels: Vec<String>,
    #[garde(dive)]
    #[serde(default)]
    outputs: Vec<Output>,
}

#[derive(Debug, Deserialize, Validate, PartialEq)]
struct Output {
    #[garde(custom(inside_the_repo))]
    path: PathBuf,
    #[garde(custom(one_file_name))]
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
fn a_valid_file_loads() {
    let input = Input::new(
        "ver.yaml",
        indoc! {"
        name: release
        labels: [ship]
    "},
    );
    let config: Config = input.load().unwrap();
    assert_eq!(config.name, "release");
    assert_eq!(config.labels, ["ship"]);
}

#[test]
fn every_schema_problem_is_reported_on_its_line() {
    let input = Input::new(
        "ver.yaml",
        indoc! {"
        name: ''
        labels: []
        outputs:
          - path: docs
            file: a.md
          - path: ../outside
            file: nested/b.md
    "},
    );
    let error = input.load::<Config>().unwrap_err();
    let expected = vec![
        problem(1, 7, "name", "cannot be empty — it names the release"),
        problem(2, 9, "labels", "must declare at least one label"),
        problem(6, 11, "outputs[1].path", "must stay inside the repository"),
        problem(7, 11, "outputs[1].file", "must be one file name"),
    ];
    let mut problems = error.problems.clone();
    problems.sort_by_key(|problem| problem.line);
    assert_eq!(problems, expected);
    let shown = error.to_string();
    assert!(
        shown.contains("ver.yaml:6:11: outputs[1].path: must stay inside the repository"),
        "{shown}"
    );
}

#[test]
fn the_reader_refuses_loose_booleans_duplicate_keys_and_merge_keys() {
    type Flags = BTreeMap<String, bool>;
    let loose = Input::new(
        "a.yaml",
        indoc! {"
        flag: yes
    "},
    );
    assert!(loose.parse::<Flags>().is_err());
    let duplicate = Input::new(
        "b.yaml",
        indoc! {"
        flag: true
        flag: false
    "},
    );
    let error = duplicate.parse::<Flags>().unwrap_err();
    assert_eq!(error.problems[0].line, Some(2), "{error}");
    let merged = Input::new(
        "c.yaml",
        indoc! {"
        base: &base
          flag: true
        copy:
          <<: *base
    "},
    );
    assert!(merged.parse::<BTreeMap<String, Flags>>().is_err());
}

#[test]
fn frontmatter_problems_keep_the_files_line_numbers() {
    #[derive(Debug, Deserialize, Validate)]
    struct Fragment {
        #[garde(custom(cannot_be_empty("it names the package")))]
        package: String,
    }
    let file = Input::new(
        ".changeset/a.md",
        indoc! {"
        ---
        package: ''
        ---

        A summary.
    "},
    );
    let (matter, body) = file.frontmatter().unwrap();
    assert_eq!(body.trim(), "A summary.");
    let error = matter.load::<Fragment>().unwrap_err();
    assert_eq!(error.problems[0].line, Some(2), "{error}");
    let plain = Input::new(
        "b.md",
        indoc! {"
        No frontmatter here.
    "},
    );
    assert!(plain.frontmatter().is_none());
}

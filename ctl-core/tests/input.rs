//! Declared input: parse, validate once, and place every problem.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "input")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use ctl_core::indoc;
use ctl_core::input::{FieldPath, Input, Limits, Place, Problem, Written};
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
    let list = Input::new(
        ".changeset/b.md",
        indoc! {"
        ---
        - package
        ---
    "},
    );
    let (matter, _) = list.frontmatter().unwrap();
    let error = matter.parse::<Fragment>().unwrap_err();
    assert_eq!(error.problems[0].line, Some(2), "{error}");
    assert!(!error.problems[0].message.contains("line"), "{error}");
    let plain = Input::new(
        "b.md",
        indoc! {"
        No frontmatter here.
    "},
    );
    assert!(plain.frontmatter().is_none());
}

#[test]
fn an_alias_error_names_where_its_value_is_defined_in_the_file() {
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Copied {
        base: BTreeMap<String, String>,
        copy: Port,
    }
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Port {
        port: u16,
    }
    let file = Input::new(
        "a.md",
        indoc! {"
        ---
        base: &b
          port: eighty
        copy: *b
        ---
    "},
    );
    let (matter, _) = file.frontmatter().unwrap();
    let error = matter.parse::<Copied>().unwrap_err();
    assert_eq!(error.problems[0].line, Some(4), "{error}");
    assert_eq!(error.problems[0].column, Some(7), "{error}");
    assert_eq!(
        error.problems[0].message,
        "invalid u16 (defined at line 3, column 9)"
    );
}

#[test]
fn an_alias_reached_through_another_alias_names_the_failing_value() {
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Copied {
        base: BTreeMap<String, String>,
        outer: BTreeMap<String, BTreeMap<String, String>>,
        copy: Wrapped,
    }
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Wrapped {
        nested: Port,
    }
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Port {
        port: u16,
    }
    let file = Input::new(
        "a.md",
        indoc! {"
        ---
        base: &b
          port: eighty
        outer: &o
          nested: *b
        copy: *o
        ---
    "},
    );
    let (matter, _) = file.frontmatter().unwrap();
    let error = matter.parse::<Copied>().unwrap_err();
    assert_eq!(error.problems[0].line, Some(6), "{error}");
    assert_eq!(error.problems[0].column, Some(7), "{error}");
    assert_eq!(
        error.problems[0].message,
        "invalid u16 (defined at line 3, column 9)"
    );
}

#[test]
fn a_path_that_leaves_the_repository_is_refused() {
    for outside in ["/etc/passwd", "../up", "docs/../../up"] {
        assert!(
            inside_the_repo(std::path::Path::new(outside), &()).is_err(),
            "{outside}"
        );
    }
    for inside in ["docs", "docs/a.md", "./docs"] {
        assert!(
            inside_the_repo(std::path::Path::new(inside), &()).is_ok(),
            "{inside}"
        );
    }
}

/// What a user reads for one malformed file.
fn report<T: serde::de::DeserializeOwned>(name: &str, text: &str) -> String {
    match Input::new(name, text).parse::<T>() {
        Ok(_) => format!("{name}: parsed"),
        Err(error) => error.to_string(),
    }
}

/// Every problem report the loader writes from serde-saphyr's wording, one
/// line each. The three ledgers repeat the files a real qctl run reported
/// on. Control characters in a value or key come out escaped.
#[test]
fn problem_reports_keep_their_wording() {
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Ledger {
        schema_version: u32,
        prefix: String,
        #[serde(default)]
        queue: Vec<BTreeMap<String, String>>,
    }
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Port {
        port: u16,
    }
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Strict {
        name: String,
    }
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum Mode {
        Fast,
    }
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Moded {
        mode: Mode,
    }
    #[derive(Debug, Deserialize)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Copied {
        base: BTreeMap<String, String>,
        copy: Port,
    }
    type Flags = BTreeMap<String, bool>;
    let reports = [
        report::<Ledger>(
            "twice.yaml",
            indoc! {"
            schema_version: 5
            prefix: QCTL
            prefix: QCTL
            queue: []
        "},
        ),
        report::<Ledger>(
            "block.yaml",
            indoc! {"
            schema_version: 5
            prefix: QCTL
            queue: [
              - id: QCTL-001
            ]
        "},
        ),
        report::<Ledger>(
            "open.yaml",
            indoc! {"
            schema_version: 5
            prefix: QCTL
            queue: [
        "},
        ),
        report::<Flags>("loose.yaml", "flag: yes\n"),
        report::<BTreeMap<String, Flags>>(
            "merge.yaml",
            indoc! {"
            base: &base
              flag: true
            copy:
              <<: *base
        "},
        ),
        report::<Port>("port.yaml", "port: eighty\n"),
        report::<Strict>("strict.yaml", "name: a\n\"ex\\u2028tra\": b\n"),
        report::<Moded>("mode.yaml", "mode: \"sl\\aow\"\n"),
        report::<Copied>(
            "alias.yaml",
            indoc! {"
            base: &b
              port: eighty
            copy: *b
        "},
        ),
    ];
    insta::assert_snapshot!("problem_reports", reports.join("\n"));
}

#[derive(Debug, Deserialize, Validate, PartialEq)]
struct Globs {
    #[garde(dive)]
    files: BTreeMap<String, Entry>,
}

#[derive(Debug, Deserialize, Validate, PartialEq)]
struct Entry {
    #[garde(custom(cannot_be_empty("it names the label")))]
    label: String,
}

const GLOBS: &str = "\
files:
  'docs/**/*.md':
    label: ''
  src/lib.rs:
    label: ''
  a[b]:
    label: ''
";

fn glob_label(glob: &str) -> FieldPath {
    FieldPath::root().key("files").key(glob).key("label")
}

#[test]
fn a_problem_under_a_key_that_holds_dots_stars_and_slashes_lands_on_that_key() {
    let error = Input::new("globs.yaml", GLOBS).load::<Globs>().unwrap_err();
    let message = "cannot be empty — it names the label";
    let mut problems = error.problems.clone();
    problems.sort_by_key(|problem| problem.line);
    assert_eq!(
        problems,
        [
            problem(3, 12, "files.docs/**/*.md.label", message),
            problem(5, 12, "files.src/lib.rs.label", message),
            problem(7, 12, "files.a[b].label", message),
        ]
    );
}

#[test]
fn a_later_check_is_placed_through_place() {
    let input = Input::new("globs.yaml", GLOBS);
    let at = input.place(&glob_label("docs/**/*.md")).unwrap();
    assert_eq!((at.line, at.column), (3, 12));
    let at = input.place(&glob_label("a[b]")).unwrap();
    assert_eq!((at.line, at.column), (7, 12));
    assert_eq!(
        input.problem_at(&glob_label("docs/**/*.md"), "is not a known label"),
        problem(3, 12, "files.docs/**/*.md.label", "is not a known label")
    );
}

fn pair(place: Option<Place>) -> Option<(usize, usize)> {
    place.map(|at| (at.line, at.column))
}

#[test]
fn a_key_is_placed_where_it_is_written_and_its_value_where_it_starts() {
    let input = Input::new("globs.yaml", GLOBS);
    let glob = FieldPath::root().key("files").key("docs/**/*.md");
    assert_eq!(
        pair(input.place(&glob)),
        Some((3, 5)),
        "a block mapping starts below its key"
    );
    assert_eq!(pair(input.place_key(&glob)), Some((2, 3)));
    let label = glob_label("docs/**/*.md");
    assert_eq!(pair(input.place(&label)), Some((3, 12)));
    assert_eq!(pair(input.place_key(&label)), Some((3, 5)));
    let bracket = FieldPath::root().key("files").key("a[b]");
    assert_eq!(pair(input.place_key(&bracket)), Some((6, 3)));
}

#[test]
fn a_path_the_file_lacks_gives_the_key_of_its_nearest_ancestor() {
    let input = Input::new("globs.yaml", GLOBS);
    let missing = FieldPath::root().key("files").key("missing").key("label");
    assert_eq!(pair(input.place_key(&missing)), Some((1, 1)));
    assert_eq!(
        pair(input.place_key(&FieldPath::root())),
        pair(input.place(&FieldPath::root()))
    );
}

#[test]
fn an_item_has_no_key_and_a_flow_mapping_is_placed_on_its_line() {
    let input = Input::new(
        "list.yaml",
        "vocab:\n  - a\n  - b\nflow: {styles: [x], vocab: []}\n",
    );
    let list = FieldPath::root().key("vocab");
    assert_eq!(pair(input.place_key(&list)).map(|at| at.0), Some(1));
    assert_eq!(pair(input.place(&list)).map(|at| at.0), Some(2));
    let later = list.index(1);
    assert_eq!(pair(input.place_key(&later)).map(|at| at.0), Some(3));
    assert_eq!(input.place_key(&later), input.place(&later));
    let flow = FieldPath::root().key("flow");
    assert_eq!(
        pair(input.place_key(&flow.clone().key("styles"))),
        Some((4, 8))
    );
    assert_eq!(pair(input.place_key(&flow.key("vocab"))), Some((4, 21)));
}

#[test]
fn a_path_the_file_lacks_is_placed_at_its_nearest_ancestor() {
    let input = Input::new("globs.yaml", GLOBS);
    let files = input.place(&FieldPath::root().key("files")).unwrap();
    assert_eq!(input.place(&glob_label("docs/**/*.rs")), Some(files));
    assert_eq!(
        input.place(&FieldPath::root().key("files").key("missing")),
        Some(files)
    );
}

#[test]
fn a_text_that_does_not_parse_places_nothing() {
    let input = Input::new("bad.yaml", "files: [\n");
    assert_eq!(input.place(&glob_label("docs/**/*.md")), None);
    let problem = input.problem_at(&glob_label("docs/**/*.md"), "is not a known label");
    assert_eq!((problem.line, problem.column), (None, None));
}

#[test]
fn a_frontmatter_place_keeps_the_line_of_the_whole_file() {
    let file = Input::new(
        "note.md",
        "---\nfiles:\n  'a.b':\n    label: x\n---\nbody\n",
    );
    let (matter, _) = file.frontmatter().unwrap();
    let at = matter.place(&glob_label("a.b")).unwrap();
    assert_eq!((at.line, at.column), (4, 12));
}

/// A release schema whose targets and jobs are flattened maps, as verctl's
/// `assets` holds its targets.
#[derive(Debug, Deserialize, Validate)]
struct Shipped {
    #[garde(dive)]
    assets: Assets,
    #[garde(dive)]
    #[serde(default)]
    ci: Jobs,
}

#[derive(Debug, Deserialize, Validate)]
struct Assets {
    #[serde(flatten)]
    #[garde(dive)]
    targets: BTreeMap<String, Target>,
}

#[derive(Debug, Default, Deserialize, Validate)]
struct Jobs {
    #[serde(flatten)]
    #[garde(dive)]
    jobs: BTreeMap<String, Target>,
}

#[derive(Debug, Deserialize, Validate)]
struct Target {
    #[garde(custom(at_least_one("file")))]
    files: Vec<String>,
}

const SHIPPED: &str = indoc! {"
    assets:
      linux-x64:
        files: []
      a.b/c:
        files: []
      targets:
        files: []
    ci:
      lint:
        files: []
"};

/// `problems` ordered by place and field.
fn ordered(mut problems: Vec<Problem>) -> Vec<Problem> {
    problems.sort_by_key(|problem| (problem.line, problem.column, problem.field.clone()));
    problems
}

fn assets_targets() -> FieldPath {
    FieldPath::root().key("assets").key("targets")
}

#[test]
fn a_problem_under_a_declared_flattened_field_sits_where_the_file_writes_it() {
    let error = Input::new("ver.yaml", SHIPPED)
        .flattened(assets_targets())
        .flattened(FieldPath::root().key("ci").key("jobs"))
        .load::<Shipped>()
        .unwrap_err();
    let message = "must declare at least one file";
    assert_eq!(
        ordered(error.problems),
        [
            problem(3, 12, "assets.linux-x64.files", message),
            problem(5, 12, "assets.a.b/c.files", message),
            problem(7, 12, "assets.targets.files", message),
            problem(10, 12, "ci.lint.files", message),
        ]
    );
}

/// Undeclared, each `assets` path walks into the target named `targets`, the
/// one key `targets` the file holds, and stops there.
#[test]
fn an_undeclared_or_unmatched_flattened_field_places_as_garde_names_it() {
    let message = "must declare at least one file";
    let undeclared = ordered(
        Input::new("ver.yaml", SHIPPED)
            .load::<Shipped>()
            .unwrap_err()
            .problems,
    );
    assert_eq!(
        undeclared,
        [
            problem(7, 5, "assets.targets.a.b/c.files", message),
            problem(7, 5, "assets.targets.linux-x64.files", message),
            problem(7, 5, "assets.targets.targets.files", message),
            problem(9, 3, "ci.jobs.lint.files", message),
        ]
    );
    let unmatched = ordered(
        Input::new("ver.yaml", SHIPPED)
            .flattened(FieldPath::root().key("release").key("targets"))
            .load::<Shipped>()
            .unwrap_err()
            .problems,
    );
    assert_eq!(unmatched, undeclared);
}

/// The key, the value as written, how it is written and the string, of each
/// top-level entry.
fn listing(input: &Input) -> Vec<(String, String, Written, Option<String>)> {
    input
        .top_level()
        .unwrap_or_default()
        .into_iter()
        .map(|entry| {
            (
                entry.key,
                input.text()[entry.value].to_owned(),
                entry.written,
                entry.string,
            )
        })
        .collect()
}

fn row(
    key: &str,
    value: &str,
    written: Written,
    string: Option<&str>,
) -> (String, String, Written, Option<String>) {
    (
        key.to_owned(),
        value.to_owned(),
        written,
        string.map(str::to_owned),
    )
}

#[test]
fn the_top_level_lists_each_entry_with_its_span_style_and_string() {
    let input = Input::new(
        "page.yaml",
        indoc! {r#"
        plain: a plain value
        single: 'it''s'
        double: "tab\there"
        folded: >
          one
          two
        literal: |
          line
        number: 12
        flag: true
        nothing: null
        yes_text: yes
        list: [a, b]
        map:
          k: v
        "#},
    );
    assert_eq!(
        listing(&input),
        [
            row(
                "plain",
                "a plain value",
                Written::Plain,
                Some("a plain value")
            ),
            row("single", "'it''s'", Written::SingleQuoted, Some("it's")),
            row(
                "double",
                r#""tab\there""#,
                Written::DoubleQuoted,
                Some("tab\there")
            ),
            row(
                "folded",
                ">\n  one\n  two",
                Written::Folded,
                Some("one two\n")
            ),
            row("literal", "|\n  line", Written::Literal, Some("line\n")),
            row("number", "12", Written::Plain, None),
            row("flag", "true", Written::Plain, None),
            row("nothing", "null", Written::Plain, None),
            row("yes_text", "yes", Written::Plain, Some("yes")),
            row("list", "[a, b]", Written::Collection, None),
            row("map", "k: v", Written::Collection, None),
        ]
    );
}

#[test]
fn an_anchor_an_alias_and_a_tag_leave_the_span_of_the_value() {
    let input = Input::new(
        "page.yaml",
        indoc! {"
        a: &x text
        b: *x
        c: !!str 12
        d: &y !!str t
        "},
    );
    assert_eq!(
        listing(&input),
        [
            row("a", "text", Written::Plain, Some("text")),
            row("b", "*x", Written::Alias, Some("text")),
            row("c", "12", Written::Plain, Some("12")),
            row("d", "t", Written::Plain, Some("t")),
        ]
    );
}

#[test]
fn a_key_with_an_anchor_or_a_tag_keeps_its_entry() {
    let input = Input::new(
        "page.yaml",
        indoc! {r#"
        &a plain: 1
        &b 'single': 2
        !!str "double": 3
        !!str 4: 4
        &c !!str both: 5
        !!str &d reversed: 6
        ? &e explicit
        : 7
        "#},
    );
    assert_eq!(
        listing(&input),
        [
            row("plain", "1", Written::Plain, None),
            row("single", "2", Written::Plain, None),
            row("double", "3", Written::Plain, None),
            row("4", "4", Written::Plain, None),
            row("both", "5", Written::Plain, None),
            row("reversed", "6", Written::Plain, None),
            row("explicit", "7", Written::Plain, None),
        ]
    );
}

#[test]
fn a_flow_root_key_with_an_anchor_or_a_tag_keeps_its_entry() {
    let input = Input::new("page.yaml", "{ &a x: 1, !!str y: 2, &b !!str 'z': 3 }\n");
    let keys: Vec<_> = listing(&input).into_iter().map(|row| row.0).collect();
    assert_eq!(keys, ["x", "y", "z"]);
}

#[test]
fn a_non_finite_float_is_a_value_that_is_no_string() {
    let input = Input::new(
        "page.yaml",
        indoc! {"
        up: .inf
        down: -.inf
        none: .NaN
        after: text
        "},
    );
    assert_eq!(
        listing(&input),
        [
            row("up", ".inf", Written::Plain, None),
            row("down", "-.inf", Written::Plain, None),
            row("none", ".NaN", Written::Plain, None),
            row("after", "text", Written::Plain, Some("text")),
        ]
    );
}

#[test]
fn parse_still_refuses_a_non_finite_float_it_cannot_type() {
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    #[expect(
        dead_code,
        reason = "The test reads the refusal, and no variant is read."
    )]
    enum Anything {
        Text(String),
        Number(f64),
    }
    let input = Input::new("page.yaml", "up: .inf\n");
    let refused = input
        .parse::<BTreeMap<String, Anything>>()
        .unwrap_err()
        .to_string();
    assert!(refused.contains("non-finite"), "{refused}");
}

#[test]
fn a_key_that_is_no_string_is_left_out() {
    let input = Input::new(
        "page.yaml",
        indoc! {"
        1: one
        'two': 2
        z: w
        "},
    );
    let keys: Vec<_> = listing(&input).into_iter().map(|row| row.0).collect();
    assert_eq!(keys, ["two", "z"]);
}

#[test]
fn a_key_of_another_type_does_not_take_the_span_of_a_string_key() {
    let input = Input::new(
        "page.yaml",
        indoc! {"
        1: one
        '1': two
        null: n
        'null': m
        true: t
        "},
    );
    assert_eq!(
        listing(&input),
        [
            row("1", "two", Written::Plain, Some("two")),
            row("null", "m", Written::Plain, Some("m")),
        ]
    );
}

#[test]
fn a_sequence_or_mapping_as_a_key_is_left_out_and_drops_nothing_after_it() {
    let input = Input::new(
        "page.yaml",
        indoc! {"
        ? [a, b]
        : v
        z: w
        ? {c: d}
        : u
        y: x
        "},
    );
    assert_eq!(
        listing(&input),
        [
            row("z", "w", Written::Plain, Some("w")),
            row("y", "x", Written::Plain, Some("x")),
        ]
    );
}

#[test]
fn an_alias_as_a_key_is_left_out_and_drops_nothing_after_it() {
    let input = Input::new(
        "page.yaml",
        indoc! {"
        k: &k key
        *k : v
        z: w
        "},
    );
    assert_eq!(
        listing(&input),
        [
            row("k", "key", Written::Plain, Some("key")),
            row("z", "w", Written::Plain, Some("w")),
        ]
    );
}

#[test]
fn a_root_that_is_no_mapping_has_no_entries() {
    for text in ["- a\n- b\n", "just text\n", "", "12\n"] {
        let input = Input::new("page.yaml", text);
        assert_eq!(input.top_level().unwrap(), []);
    }
}

#[test]
fn a_duplicate_key_and_a_merge_key_are_refused_as_parse_refuses_them() {
    for text in ["a: 1\na: 2\n", "base: &b {x: 1}\n<<: *b\n"] {
        let input = Input::new("page.yaml", text);
        let refused = input.top_level().unwrap_err();
        let parsed = input.parse::<serde::de::IgnoredAny>().unwrap_err();
        assert_eq!(refused, parsed);
    }
}

#[test]
fn a_text_past_a_limit_is_refused_with_the_limit_named() {
    let limits = Limits {
        max_depth: Some(2),
        ..Limits::NONE
    };
    let input = Input::new("page.yaml", "a: {b: {c: {d: 1}}}\n").limits(limits);
    let refused = input.top_level().unwrap_err();
    assert!(refused.to_string().contains("max_depth"), "{refused}");
}

#[test]
fn a_frontmatter_entry_is_placed_in_the_whole_file() {
    let file = Input::new("note.md", "---\ntitle: A page\n---\nbody\n");
    let (matter, _) = file.frontmatter().unwrap();
    let entries = matter.top_level().unwrap();
    assert_eq!(&matter.text()[entries[0].value.clone()], "A page");
}

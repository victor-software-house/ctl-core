//! Served files: a template declares its own target, and writing it sets the
//! one mode bit git records.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "served", feature = "fixture"))]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use ctl_core::fixture::Suite;
use ctl_core::indoc;
use ctl_core::served::{Served, render};

fn versions() -> BTreeMap<&'static str, BTreeMap<&'static str, &'static str>> {
    BTreeMap::from([("versions", BTreeMap::from([("qctl", "0.5.0")]))])
}

fn served(source: &str) -> Result<Served, String> {
    render(
        Path::new(".ctl/templates/q.jinja"),
        source,
        ".jinja",
        &versions(),
    )
    .map_err(|error| error.to_string())
}

#[test]
fn the_declarations_decide_where_a_template_lands() {
    let cases = [
        ("nothing declared: its own name, at the root", "", "q"),
        (
            "a directory",
            r#"{%- set path = "tasks/q" -%}"#,
            "tasks/q/q",
        ),
        ("a name", r#"{%- set name = "mise.toml" -%}"#, "mise.toml"),
        (
            "both",
            indoc! {r#"
                {%- set path = "examples" -%}
                {%- set name = "mise.toml" -%}
            "#},
            "examples/mise.toml",
        ),
        (
            "a computed declaration",
            r#"{%- set path = "tasks/" ~ "q" -%}"#,
            "tasks/q/q",
        ),
    ];
    for (label, declaration, target) in cases {
        let served = served(&format!("{declaration}served\n")).unwrap();
        assert_eq!(served.target(), Path::new(target), "{label}");
        assert_eq!(served.body(), "served\n", "{label}");
        assert!(!served.executable(), "{label}");
    }
}

#[test]
fn a_declaration_that_does_not_fit_names_the_template_and_the_field() {
    let cases = [
        (
            r#"{%- set path = "../escaped" -%}"#,
            "path: must stay inside the repository",
        ),
        (
            r#"{%- set path = "/etc" -%}"#,
            "path: must stay inside the repository",
        ),
        (
            r#"{%- set name = "tasks/q" -%}"#,
            "name: must be one file name",
        ),
        (r#"{%- set name = ".." -%}"#, "name: must be one file name"),
        (r"{%- set path = 5 -%}", "invalid type"),
        (r#"{%- set executable = "yes" -%}"#, "expected a boolean"),
    ];
    for (declaration, complaint) in cases {
        let report = served(&format!("{declaration}served\n")).unwrap_err();
        assert!(report.contains(complaint), "{declaration}: {report}");
        assert!(
            report.starts_with(".ctl/templates/q.jinja: "),
            "{declaration}: {report}"
        );
    }
}

#[test]
fn an_export_the_schema_does_not_know_is_the_templates_own() {
    let served = served(indoc! {r#"
        {%- set tool = "github:victor-software-house/qctl" -%}
        {%- set path = "tasks/q" -%}
        {{ tool }} = "{{ versions["qctl"] }}"
    "#})
    .unwrap();
    assert_eq!(served.target(), Path::new("tasks/q/q"));
    assert_eq!(
        served.body(),
        "github:victor-software-house/qctl = \"0.5.0\"\n"
    );
}

#[test]
fn a_shebang_stays_on_line_one_and_the_trailing_newline_stays() {
    let served = served(indoc! {r#"
        {%- set executable = true -%}
        #!/usr/bin/env bash
        exec qctl@{{ versions["qctl"] }} "$@"
    "#})
    .unwrap();
    assert!(served.executable());
    assert_eq!(
        served.body(),
        indoc! {r#"
            #!/usr/bin/env bash
            exec qctl@0.5.0 "$@"
        "#}
    );
}

#[test]
fn an_undefined_name_fails_instead_of_rendering_empty() {
    let report = served("{{ versions[\"verctl\"] }}\n").unwrap_err();
    assert!(report.contains("render:"), "{report}");
}

#[test]
fn a_template_without_the_suffix_is_refused() {
    let report = render(Path::new("q.txt"), "x\n", ".jinja", &versions())
        .unwrap_err()
        .to_string();
    assert!(report.contains("does not end in .jinja"), "{report}");
}

#[test]
fn ctl_core_fragments_are_available() {
    let served = served(indoc! {r#"
        {%- from "ctl/version.md.jinja" import version_line -%}
        {{ version_line(versions["qctl"]) }}
    "#})
    .unwrap();
    assert_eq!(served.body(), "version: 0.5.0\n");
}

#[cfg(unix)]
#[test]
fn writing_creates_the_directory_and_sets_the_one_mode_bit() {
    use std::os::unix::fs::PermissionsExt;

    let suite = Suite::new("served");
    let root = suite.scenario("modes");
    let task = served(indoc! {r#"
        {%- set path = "tasks/q" -%}
        {%- set executable = true -%}
        #!/usr/bin/env bash
    "#})
    .unwrap();
    let readme = served(r#"{%- set name = "README.md" -%}Run qctl."#).unwrap();
    let stale = root.file("README.md", "old");
    fs::set_permissions(&stale, fs::Permissions::from_mode(0o755)).unwrap();

    let written = task.write(root.path()).unwrap();
    readme.write(root.path()).unwrap();

    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(written, root.path().join("tasks/q/q"));
    assert_eq!(mode(&written), 0o755);
    assert_eq!(mode(&stale), 0o644);
    assert_eq!(root.read("README.md"), "Run qctl.");
}

#[cfg(unix)]
#[test]
fn a_symlinked_target_is_refused() {
    let suite = Suite::new("served");
    let root = suite.scenario("symlink");
    let outside = suite.root().file("outside.md", "keep");
    std::os::unix::fs::symlink(&outside, root.path().join("q")).unwrap();

    let report = served("x\n").unwrap().write(root.path()).unwrap_err();
    assert!(
        report.to_string().contains("must not be a symlink"),
        "{report}"
    );
    assert_eq!(fs::read_to_string(&outside).unwrap(), "keep");
}

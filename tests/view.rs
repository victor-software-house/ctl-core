//! Envelope JSON and `Render` + formatdoc.
#![allow(missing_docs)]
#![cfg(feature = "view")]

use ctl_core::{
    ColorMode, Envelope, ErrorBody, OutputFormat, Pretty, Render, View, formatdoc, indoc,
    render_template,
};
use serde::Serialize;

#[test]
fn json_view_is_the_model() {
    let view = View::new(OutputFormat::Json, ColorMode::Never);
    let env = Envelope::ok(["demo"]);
    let encoded = serde_json::to_string(&env).unwrap();
    assert!(encoded.contains("\"status\":\"ok\""));
    assert!(encoded.contains("demo"));
    let _ = view;
}

struct Demo {
    name: &'static str,
    to: &'static str,
}

impl Render for Demo {
    fn render_pretty(&self) -> String {
        formatdoc! {"
            bump    {name} -> {to}
            dry-run (no files written)
            ",
            name = self.name,
            to = self.to,
        }
    }
}

#[test]
fn pretty_uses_formatdoc() {
    let demo = Demo {
        name: "verctl",
        to: "0.0.1",
    };
    let pretty = demo.render_pretty();
    assert!(pretty.contains("bump    verctl -> 0.0.1"));
    assert!(!pretty.contains('{'));
    let view = View::new(OutputFormat::Pretty, ColorMode::Never);
    assert_eq!(view.format, OutputFormat::Pretty);
}

#[test]
fn envelope_ok_tag() {
    let json = serde_json::to_value(Envelope::ok("demo")).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["schema_version"], 1);
}

#[test]
fn envelope_err_tag() {
    let json = serde_json::to_value(Envelope::<()>::err(ErrorBody::new("toy", "nope"))).unwrap();
    assert_eq!(json["status"], "err");
    assert_eq!(json["error"]["bin"], "toy");
}

#[derive(Serialize)]
struct PublishDemo {
    crates: Vec<String>,
    release: Option<String>,
    dry_run: bool,
}

impl Pretty for PublishDemo {
    const TEMPLATE: &'static str = indoc! {"
        {% for entry in crates -%}
        crate   {{ entry }}
        {% endfor -%}
        {% if release -%}
        release {{ release }}
        {% endif -%}
        {% if dry_run -%}
        dry-run (nothing published)
        {% endif %}
    "};
}

struct PrettyCase {
    name: &'static str,
    crates: &'static [&'static str],
    release: Option<&'static str>,
    dry_run: bool,
    expected: &'static str,
}

#[test]
fn pretty_template_varies_by_data() {
    let cases = [
        PrettyCase {
            name: "crate, release, dry-run",
            crates: &["ctl-core@0.0.1 (cargo)"],
            release: Some("would create v0.0.1"),
            dry_run: true,
            expected: indoc! {"
                crate   ctl-core@0.0.1 (cargo)
                release would create v0.0.1
                dry-run (nothing published)
            "},
        },
        PrettyCase {
            name: "two crates only",
            crates: &["ctl-core@0.0.1 (cargo)", "verctl@0.0.1 (cargo)"],
            release: None,
            dry_run: false,
            expected: indoc! {"
                crate   ctl-core@0.0.1 (cargo)
                crate   verctl@0.0.1 (cargo)
            "},
        },
        PrettyCase {
            name: "release without dry-run",
            crates: &["ctl-core@0.0.1 (cargo)"],
            release: Some("https://github.com/victor-software-house/ctl-core/releases/tag/v0.0.1"),
            dry_run: false,
            expected: indoc! {"
                crate   ctl-core@0.0.1 (cargo)
                release https://github.com/victor-software-house/ctl-core/releases/tag/v0.0.1
            "},
        },
        PrettyCase {
            name: "empty model is empty pretty",
            crates: &[],
            release: None,
            dry_run: false,
            expected: "",
        },
        PrettyCase {
            name: "dry-run only",
            crates: &[],
            release: None,
            dry_run: true,
            expected: "dry-run (nothing published)\n",
        },
    ];
    for case in cases {
        let demo = PublishDemo {
            crates: case
                .crates
                .iter()
                .map(|entry| (*entry).to_owned())
                .collect(),
            release: case.release.map(str::to_owned),
            dry_run: case.dry_run,
        };
        let pretty = render_template(PublishDemo::TEMPLATE, &demo).expect(case.name);
        assert_eq!(pretty, case.expected, "{}", case.name);
    }
}

#[test]
fn quiet_builder() {
    assert!(
        View::new(OutputFormat::Pretty, ColorMode::Never)
            .quiet(true)
            .quiet
    );
}

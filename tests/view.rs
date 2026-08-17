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

#[test]
fn pretty_template_owns_the_ifs() {
    let demo = PublishDemo {
        crates: vec!["ctl-core@0.0.1 (cargo)".into()],
        release: Some("would create v0.0.1".into()),
        dry_run: true,
    };
    let pretty = render_template(PublishDemo::TEMPLATE, &demo).unwrap();
    assert_eq!(
        pretty,
        indoc! {"
            crate   ctl-core@0.0.1 (cargo)
            release would create v0.0.1
            dry-run (nothing published)
        "}
    );
}

#[test]
fn quiet_builder() {
    assert!(
        View::new(OutputFormat::Pretty, ColorMode::Never)
            .quiet(true)
            .quiet
    );
}

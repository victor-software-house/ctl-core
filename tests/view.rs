#![allow(missing_docs)]
#![cfg(feature = "view")]

use ctl_core::{ColorMode, Envelope, OutputFormat, Render, View, formatdoc};

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

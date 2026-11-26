//! The registry: its order, its builders, how it finds a format for a file,
//! and a format written outside the crate.

use std::borrow::Cow;
use std::path::Path;

use ctl_input::format::{Fault, Format, Mismatch, Reading, Refusal, Resolved, TOML, Wanted, YAML};
use ctl_input::tree::{Node, NodeKind, Scalar, ScalarStyle, Tag};
use ctl_input::{Formats, Limits};

use crate::common::{entries, get, read, source, value};

/// `key=value` lines into a flat mapping: one span per key and per value.
struct Kv;

static KV: Kv = Kv;

impl Format for Kv {
    fn name(&self) -> &'static str {
        "kv"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["kv"]
    }

    fn read(&self, text: &str, _limits: &Limits, _reading: Reading) -> Result<Node, Refusal> {
        let mut pairs = Vec::new();
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let content = line.trim_end_matches('\n');
            let Some((key, value)) = content.split_once('=') else {
                let span = offset..offset + content.len();
                let detail = "expected key=value".to_owned();
                return Err(Refusal::new(Fault::Syntax { detail }, Some(span), None));
            };
            let key_span = offset..offset + key.len();
            let value_start = offset + key.len() + 1;
            let value_span = value_start..value_start + value.len();
            pairs.push((
                Node::scalar(key_span, Scalar::new(key, ScalarStyle::Plain)),
                Node::scalar(value_span, Scalar::new(value, ScalarStyle::Plain)),
            ));
            offset += line.len();
        }
        Ok(Node::mapping(0..text.len(), pairs))
    }

    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "Wanted is non_exhaustive, so a match outside the crate needs a wildcard"
    )]
    fn resolve<'a>(
        &self,
        scalar: &'a Scalar,
        _tag: Option<&Tag>,
        wanted: Wanted,
    ) -> Result<Resolved<'a>, Mismatch> {
        let text = scalar.value();
        match wanted {
            Wanted::Any | Wanted::Str => Ok(Resolved::Str(Cow::Borrowed(text))),
            Wanted::Int => text
                .parse()
                .map(Resolved::Int)
                .map_err(|_error| Mismatch::new(wanted)),
            _ => Err(Mismatch::new(wanted)),
        }
    }
}

/// A format that takes the name `yaml` and the extension `alt`.
struct Alternate;

static ALTERNATE: Alternate = Alternate;

impl Format for Alternate {
    fn name(&self) -> &'static str {
        "yaml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["alt"]
    }

    fn read(&self, text: &str, limits: &Limits, reading: Reading) -> Result<Node, Refusal> {
        YAML.read(text, limits, reading)
    }

    fn resolve<'a>(
        &self,
        scalar: &'a Scalar,
        tag: Option<&Tag>,
        wanted: Wanted,
    ) -> Result<Resolved<'a>, Mismatch> {
        YAML.resolve(scalar, tag, wanted)
    }
}

fn names(formats: &Formats) -> Vec<&'static str> {
    formats.names().collect()
}

/// The message of the one problem `formats` finds with the file `path`.
fn refusal(formats: &Formats, path: &str) -> String {
    let Err(error) = formats.of(Path::new(path)) else {
        panic!("{path} was accepted");
    };
    assert_eq!(error.problems.len(), 1);
    assert_eq!(error.problems[0].line, None);
    error.problems[0].message.clone()
}

fn accepted(formats: &Formats, path: &str) -> &'static str {
    formats
        .of(Path::new(path))
        .unwrap_or_else(|error| panic!("{error}"))
        .name()
}

#[test]
fn the_builtin_registry_holds_yaml_then_toml() {
    assert_eq!(names(&Formats::builtin()), ["yaml", "toml"]);
    let formats = Formats::builtin();
    assert_eq!(formats.get("yaml").map(Format::name), Some("yaml"));
    assert_eq!(
        formats.get("toml").map(Format::extensions),
        Some(&["toml"][..])
    );
    assert!(formats.get("json").is_none());
}

#[test]
fn a_format_is_a_trait_object_a_registry_can_hold() {
    let held: Vec<&'static dyn Format> = vec![&YAML, &TOML, &KV];
    let names: Vec<_> = held.iter().map(|format| format.name()).collect();
    assert_eq!(names, ["yaml", "toml", "kv"]);
}

#[test]
fn adding_a_name_that_is_registered_replaces_it_in_place() {
    let formats = Formats::builtin().add(&ALTERNATE);
    assert_eq!(names(&formats), ["yaml", "toml"]);
    assert_eq!(accepted(&formats, "app.alt"), "yaml");
    assert_eq!(
        refusal(&formats, "app.yaml"),
        "unsupported extension .yaml; name the file *.alt or *.toml"
    );
}

#[test]
fn adding_a_new_name_appends_it() {
    let formats = Formats::builtin().add(&KV);
    assert_eq!(names(&formats), ["yaml", "toml", "kv"]);
}

#[test]
fn removing_a_format_leaves_the_others_in_order() {
    let formats = Formats::builtin().add(&KV).remove("yaml");
    assert_eq!(names(&formats), ["toml", "kv"]);
    assert!(formats.get("yaml").is_none());
    assert_eq!(names(&formats.remove("absent")), ["toml", "kv"]);
}

#[test]
fn an_extension_finds_its_format_without_regard_to_case() {
    let formats = Formats::builtin();
    assert_eq!(accepted(&formats, "app.yaml"), "yaml");
    assert_eq!(accepted(&formats, "app.yml"), "yaml");
    assert_eq!(accepted(&formats, "dir/APP.YAML"), "yaml");
    assert_eq!(accepted(&formats, "app.toml"), "toml");
    assert_eq!(accepted(&formats, "app.TOML"), "toml");
}

#[test]
fn a_refusal_lists_every_registered_extension_sorted() {
    let both = Formats::builtin();
    assert_eq!(
        refusal(&both, "app.json"),
        "unsupported extension .json; name the file *.toml, *.yaml, or *.yml"
    );
    assert_eq!(
        refusal(&both, "app"),
        "no file extension; name the file *.toml, *.yaml, or *.yml"
    );
    assert_eq!(
        refusal(&both.clone().remove("toml"), "app.toml"),
        "unsupported extension .toml; name the file *.yaml or *.yml"
    );
    assert_eq!(
        refusal(&both.clone().remove("yaml"), "app.yaml"),
        "unsupported extension .yaml; name the file *.toml"
    );
    assert_eq!(
        refusal(&both.add(&KV), "app.json"),
        "unsupported extension .json; name the file *.kv, *.toml, *.yaml, or *.yml"
    );
}

#[test]
fn an_empty_registry_refuses_every_extension() {
    let empty = Formats::empty();
    assert_eq!(names(&empty), Vec::<&str>::new());
    for path in ["app.yaml", "app.yml", "app.toml", "app.kv"] {
        let extension = path.rsplit('.').next().unwrap_or_default();
        assert_eq!(
            refusal(&empty, path),
            format!("unsupported extension .{extension}; no format is registered")
        );
    }
    assert_eq!(
        refusal(&empty, "app"),
        "no file extension; no format is registered"
    );
}

#[test]
fn the_error_names_the_file() {
    let error = Formats::builtin()
        .of(Path::new("dir/app.json"))
        .err()
        .unwrap();
    assert_eq!(error.name, "dir/app.json");
}

#[test]
fn a_format_written_outside_the_crate_is_found_read_and_resolved() {
    let formats = Formats::builtin().add(&KV);
    let format = formats.of(Path::new("app.kv")).unwrap();
    assert_eq!(format.name(), "kv");
    let text = "name=release\nport=x\nretries=3\n";
    let root = read(format, text);
    let keys: Vec<_> = entries(&root)
        .iter()
        .map(|(key, _)| source(text, key))
        .collect();
    assert_eq!(keys, ["name", "port", "retries"]);
    let port = get(&root, "port");
    assert_eq!(source(text, port), "x");
    assert_eq!(
        text[..port.span().start].matches('\n').count(),
        1,
        "line 2 of 3"
    );
    assert_eq!(
        format.resolve(scalar_of(port), None, Wanted::Int),
        Err(Mismatch::new(Wanted::Int))
    );
    let retries = scalar_of(get(&root, "retries"));
    assert_eq!(
        format.resolve(retries, None, Wanted::Int),
        Ok(Resolved::Int(3))
    );
    assert_eq!(value(get(&root, "name")), "release");
}

fn scalar_of(node: &Node) -> &Scalar {
    let NodeKind::Scalar(scalar) = node.kind() else {
        panic!("not a scalar");
    };
    scalar
}

#[test]
fn a_format_outside_the_crate_refuses_with_a_span() {
    let refusal = KV
        .read("a=1\nbroken\n", &Limits::NONE, Reading::Strict)
        .unwrap_err();
    assert_eq!(refusal.span(), Some(4..10));
    assert_eq!(refusal.defined_at(), None);
    assert!(matches!(refusal.fault(), Fault::Syntax { .. }));
}

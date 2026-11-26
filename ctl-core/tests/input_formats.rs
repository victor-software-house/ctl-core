//! Input formats as values: the same field in two formats, a CLI's own
//! format, the registry's tiers, and what a message set and a kept tree
//! promise.
//!
//! The process default is global, so the one test that installs it runs its
//! cases in order, and every other test reads through a registry of its own.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(
    feature = "app",
    feature = "fixture",
    feature = "input",
    feature = "input-toml"
))]

use std::borrow::Cow;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ctl_core::fixture::Suite;
use ctl_core::input::format::{Fault, Mismatch, Reading, Refusal, Resolved, Wanted};
use ctl_core::input::tree::{Node, Scalar, ScalarStyle, Tag};
use ctl_core::input::{
    FieldPath, Format, Formats, Input, InputError, Limits, TOML, YAML, set_default_formats,
};
use ctl_core::prelude::*;
use ctl_core::validate::cannot_be_empty;
use garde::Validate;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};

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

/// A format that reads as YAML does and counts its reads.
struct Counting(AtomicUsize);

impl Format for Counting {
    fn name(&self) -> &'static str {
        "counting"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["count"]
    }

    fn read(&self, text: &str, limits: &Limits, reading: Reading) -> Result<Node, Refusal> {
        self.0.fetch_add(1, Ordering::Relaxed);
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

#[derive(Debug, Deserialize, Validate)]
struct Release {
    #[garde(skip)]
    #[expect(dead_code, reason = "only the problems are read")]
    title: String,
    #[garde(custom(cannot_be_empty("it names the release")))]
    name: String,
}

#[derive(Debug, Deserialize)]
#[expect(dead_code, reason = "only the problems are read")]
struct Server {
    name: String,
    mode: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
#[expect(dead_code, reason = "only the problems are read")]
struct Nested {
    name: String,
    server: Host,
}

#[derive(Debug, Deserialize)]
#[expect(dead_code, reason = "only the problems are read")]
struct Host {
    host: String,
    port: u16,
}

/// The one problem of an input that fails, as line, column, field and words.
fn one(error: &InputError) -> (Option<usize>, Option<usize>, &str, &str) {
    assert_eq!(error.problems.len(), 1, "{error}");
    let problem = &error.problems[0];
    (
        problem.line,
        problem.column,
        &problem.field,
        &problem.message,
    )
}

#[test]
fn the_same_field_in_two_formats_is_one_problem_worded_alike_on_line_2() {
    let suite = Suite::new("input-formats");
    let root = suite.scenario("two");
    let yaml = root.file("app.yaml", "title: a\nname: ''\n");
    let toml = root.file("app.toml", "title = 'a'\nname = ''\n");
    let formats = Formats::builtin();
    let first = formats.open(&yaml).unwrap().load::<Release>().unwrap_err();
    let second = formats.open(&toml).unwrap().load::<Release>().unwrap_err();
    let (first, second) = (one(&first), one(&second));
    assert_eq!((first.0, first.2, first.3), (Some(2), "name", second.3));
    assert_eq!((second.0, second.2), (Some(2), "name"));
    assert_eq!(first.3, "cannot be empty — it names the release");
}

#[test]
fn a_clis_own_format_is_read_through_a_registry_and_placed_on_its_line() {
    let suite = Suite::new("input-formats");
    let root = suite.scenario("kv");
    let path = root.file("app.kv", "name=release\nmode=fast\nport=x\n");
    let input = Formats::builtin().add(&KV).open(&path).unwrap();
    let error = input.parse::<Server>().unwrap_err();
    assert_eq!(
        one(&error),
        (Some(3), Some(6), "", "expected an integer"),
        "{error}"
    );
    let at = input
        .place(&FieldPath::root().key("port"))
        .map(|at| (at.line, at.column));
    assert_eq!(at, Some((3, 6)));
    let entries = input.top_level().unwrap();
    let keys: Vec<_> = entries.iter().map(|entry| entry.key.as_str()).collect();
    assert_eq!(keys, ["name", "mode", "port"]);
    assert!(
        Formats::builtin().open(&path).is_err(),
        "the built-in registry has no kv"
    );
}

#[test]
fn a_duplicate_key_is_one_problem_on_line_2_even_after_a_listing() {
    let input = Input::new("ledger.yaml", "prefix: a\nprefix: b\n");
    assert_eq!(input.top_level().unwrap().len(), 2);
    let error = input.parse::<IgnoredAny>().unwrap_err();
    assert_eq!(
        one(&error),
        (Some(2), Some(1), "", "duplicate key `prefix`"),
        "{error}"
    );
}

#[test]
fn a_duplicate_key_reads_the_same_in_yaml_and_toml() {
    let texts: [(&str, &'static dyn Format, &str); 2] = [
        ("ledger.yaml", &YAML, "prefix: a\nprefix: b\n"),
        ("ledger.toml", &TOML, "prefix = 'a'\nprefix = 'b'\n"),
    ];
    for (name, format, text) in texts {
        let error = Input::new_with_format(name, text, format)
            .parse::<IgnoredAny>()
            .unwrap_err();
        assert_eq!(
            one(&error),
            (Some(2), Some(1), "", "duplicate key `prefix`"),
            "{error}"
        );
    }
}

#[test]
fn a_missing_nested_field_sits_on_the_key_that_holds_its_mapping() {
    let input = Input::new("app.yaml", "name: a\nmode: b\nserver:\n  host: h\n");
    let error = input.parse::<Nested>().unwrap_err();
    assert_eq!(
        one(&error),
        (Some(3), Some(1), "", "missing field `port`"),
        "{error}"
    );
    let root = Input::new("app.yaml", "mode: b\n");
    let error = root.parse::<Nested>().unwrap_err();
    assert_eq!(
        one(&error),
        (Some(1), Some(1), "", "missing field `name`"),
        "{error}"
    );
}

#[test]
fn a_load_reads_the_text_once() {
    static COUNTING: Counting = Counting(AtomicUsize::new(0));
    let input = Input::new_with_format("app.count", "title: a\nname: ''\n", &COUNTING);
    let error = input.load::<Release>().unwrap_err();
    assert_eq!(one(&error).0, Some(2), "{error}");
    assert_eq!(COUNTING.0.load(Ordering::Relaxed), 1);
}

#[test]
fn an_empty_registry_refuses_every_extension_and_new_with_format_still_reads() {
    let empty = Formats::empty();
    for name in ["app.yaml", "app.toml", "app.kv"] {
        let error = empty.open(Path::new(name)).unwrap_err();
        assert_eq!(error.problems.len(), 1, "{error}");
        assert_eq!(error.problems[0].line, None);
    }
    let input = Input::new_with_format("app.toml", "name = 'a'\n", &TOML);
    assert!(input.parse::<IgnoredAny>().is_ok());
}

#[derive(Parser)]
#[command(
    name = "toy",
    version = "1.2.3",
    about = "toy",
    env_prefix = "FORMATSTOY"
)]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Open one file.
    Show {
        /// The file to open.
        file: PathBuf,
    },
}

#[derive(Serialize)]
struct Done;

impl Present for Done {
    fn present(&self) -> Document {
        Document::new().paragraph("done")
    }
}

/// The exit code of the toy CLI, and what its handler found when it opened
/// `file`: the problem or the text of the input.
fn opened_by_a_handler(app: App<Toy>, file: &Path) -> (ExitCode, Option<Result<String, String>>) {
    let found = Rc::new(RefCell::new(None));
    let seen = Rc::clone(&found);
    let args = ["toy", "show", &file.display().to_string(), "--quiet"];
    let code = app
        .view(|cli| cli.output.view())
        .run_from(args, move |cli| {
            let Cmd::Show { file } = cli.command;
            *seen.borrow_mut() = Some(
                Input::open(&file)
                    .map(|input| input.text().to_owned())
                    .map_err(|error| error.problems[0].message.clone()),
            );
            Ok(Done)
        });
    let opened = found.borrow_mut().take();
    (code, opened)
}

/// The process default is global: every case that installs it runs here, in
/// order, and the last one puts the built-in registry back.
#[test]
fn the_process_default_decides_what_open_reads() {
    let suite = Suite::new("input-formats");
    let root = suite.scenario("default");
    let toml = root.file("app.toml", "name = 'a'\n");
    let kv = root.file("app.kv", "name=a\n");

    set_default_formats(Formats::builtin().remove("toml"));
    let error = Input::open(&toml).unwrap_err();
    assert_eq!(error.problems.len(), 1, "{error}");
    assert_eq!(error.problems[0].line, None);
    assert_eq!(
        error.problems[0].message,
        "unsupported extension .toml; name the file *.yaml or *.yml"
    );

    set_default_formats(Formats::builtin());
    assert!(Input::open(&toml).is_ok());
    assert!(Input::open(&kv).is_err(), "the built-ins have no kv");

    let (code, refused) = opened_by_a_handler(App::<Toy>::new("toy"), &kv);
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(
        refused,
        Some(Err(
            "unsupported extension .kv; name the file *.toml, *.yaml, or *.yml".to_owned()
        ))
    );
    let app = App::<Toy>::new("toy").input_formats(Formats::builtin().add(&KV));
    let (code, opened) = opened_by_a_handler(app, &kv);
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(opened, Some(Ok("name=a\n".to_owned())));

    set_default_formats(Formats::builtin());
}

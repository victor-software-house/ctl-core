//! The texts in `tests/fixtures/`, read as each type the input tests read, and
//! placed at every path of their trees. A snapshot records what each gives:
//! the value or the refusal with its place and words, and the line and column
//! of each path as a value and as a key.
//!
//! The fixtures keep the cases that once told two readers apart: a block
//! scalar, a byte order mark, an alias, a TOML table written out of order,
//! and a file the format refuses. A change to how any is read or placed shows
//! as a line of the snapshot that moved.

use std::collections::BTreeMap;
use std::fmt::{self, Debug};
use std::fs;
use std::path::{Path, PathBuf};

use ctl_input::de::{Tagged, deserialize};
use ctl_input::format::Reading;
use ctl_input::tree::{Node, NodeKind};
use ctl_input::{FieldPath, Format, Formats, Input, Limits};
use serde::Deserialize;
use serde::de::{DeserializeOwned, IgnoredAny};

#[derive(Debug, Deserialize, PartialEq)]
struct Config {
    name: String,
    labels: Vec<String>,
    #[serde(default)]
    outputs: Vec<Output>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Output {
    path: PathBuf,
    file: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct TomlConfig {
    name: String,
    labels: Vec<String>,
    release: Release,
    owner: Owner,
    #[serde(default)]
    outputs: Vec<TomlOutput>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Release {
    tag: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Owner {
    name: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct TomlOutput {
    path: PathBuf,
    #[serde(default)]
    file: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Ledger {
    schema_version: u32,
    prefix: String,
    #[serde(default)]
    queue: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Port {
    port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Strict {
    name: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Fast,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Moded {
    mode: Mode,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Copied {
    base: BTreeMap<String, String>,
    copy: Port,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Nested {
    base: BTreeMap<String, String>,
    outer: BTreeMap<String, BTreeMap<String, String>>,
    copy: Wrapped,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Wrapped {
    nested: Port,
}

type Ports = BTreeMap<String, u16>;

type Names = BTreeMap<String, String>;

/// A string field a TOML datetime can fill: the `toml` crate refuses it, and
/// the tree reads its text.
#[derive(Debug, Deserialize, PartialEq)]
struct Dated {
    #[serde(default)]
    when: String,
    #[serde(default)]
    date: String,
}

type Flags = BTreeMap<String, bool>;

#[derive(Debug, Deserialize, PartialEq)]
struct Globs {
    files: BTreeMap<String, Entry>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Entry {
    label: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Shipped {
    assets: Assets,
    #[serde(default)]
    ci: Jobs,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Assets {
    #[serde(flatten)]
    targets: BTreeMap<String, Target>,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
struct Jobs {
    #[serde(flatten)]
    jobs: BTreeMap<String, Target>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Target {
    files: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum Setting {
    List(Vec<String>),
    Word(String),
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum Anything {
    Text(String),
    Number(f64),
}

type Anythings = BTreeMap<String, Anything>;

#[derive(Debug, Deserialize, PartialEq)]
struct Tags {
    styles: Tagged<Setting>,
    level: Tagged<Setting>,
}

/// A typeless value, printed as compact JSON so a line names it briefly.
#[derive(Deserialize, PartialEq)]
#[serde(transparent)]
struct Json(serde_json::Value);

impl Debug for Json {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// The fixtures, each a text of one format whose extension comes before
/// `.fixture`, so the repository's formatters leave texts that are meant to
/// be wrong alone.
fn fixtures() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let entries = fs::read_dir(directory).unwrap_or_else(|error| panic!("{error}"));
    let mut files: Vec<PathBuf> = entries
        .map(|entry| entry.unwrap_or_else(|error| panic!("{error}")).path())
        .collect();
    files.sort();
    files
}

/// The built-in format a fixture's name asks for.
fn format_of(name: &str) -> &'static dyn Format {
    Formats::builtin()
        .of(Path::new(name))
        .unwrap_or_else(|error| panic!("{error}"))
}

/// A fixture's name, as its text names its format, and its text.
fn named(file: &Path) -> Option<(String, String)> {
    let name = file.file_stem()?.to_string_lossy().into_owned();
    let extension = Path::new(&name).extension()?.to_str()?;
    matches!(extension, "yaml" | "toml").then_some(())?;
    let text = fs::read_to_string(file).unwrap_or_else(|error| panic!("{error}"));
    Some((name, text))
}

/// What reading `text` as `T` gave: the value, or the refusal with its place.
fn outcome<T: DeserializeOwned + Debug>(name: &str, text: &str) -> String {
    let format = format_of(name);
    let read = format
        .read(text, &Limits::NONE, Reading::Strict)
        .and_then(|tree| deserialize::<T>(&tree, format));
    match read {
        Ok(value) => format!("value {value:?}"),
        Err(refusal) => {
            let place = refusal.span().map_or_else(
                || "nowhere".to_owned(),
                |span| {
                    let before = &text[..span.start.min(text.len())];
                    let line = before.matches('\n').count() + 1;
                    let column = before
                        .rsplit('\n')
                        .next()
                        .unwrap_or_default()
                        .chars()
                        .count()
                        + 1;
                    format!("{line}:{column}")
                },
            );
            format!("refused {place}: {refusal}")
        }
    }
}

fn readings(name: &str, text: &str) -> Vec<String> {
    macro_rules! read {
        ($($type:ty),* $(,)?) => {
            vec![$(format!(
                "{name} as {}: {}",
                stringify!($type),
                outcome::<$type>(name, text)
            )),*]
        };
    }
    read![
        IgnoredAny, Json, Ledger, Port, Strict, Moded, Copied, Nested, Flags, Ports, Names,
        Anythings, Globs, Shipped, Tags, Config, TomlConfig, Dated,
    ]
}

#[test]
fn the_fixtures_read_as_recorded() {
    let mut lines = Vec::new();
    let mut texts = 0;
    for file in fixtures() {
        let Some((name, text)) = named(&file) else {
            continue;
        };
        texts += 1;
        lines.extend(readings(&name, &text));
    }
    assert!(texts > 40, "{texts} fixtures");
    insta::assert_snapshot!("readings", lines.join("\n"));
}

/// Every path of `node`: the path itself, and with one key it lacks.
fn paths(node: &Node, path: &FieldPath, found: &mut Vec<FieldPath>) {
    found.push(path.clone());
    found.push(path.clone().key("no such key"));
    if let NodeKind::Sequence(items) = node.kind() {
        for (index, item) in items.iter().enumerate() {
            paths(item, &path.clone().index(index), found);
        }
    }
    if let NodeKind::Mapping(entries) = node.kind() {
        for (key, value) in entries {
            if let NodeKind::Scalar(scalar) = key.kind() {
                paths(value, &path.clone().key(scalar.value()), found);
            }
        }
    }
}

/// The input for a fixture, in the format its name asks for.
fn input(name: &str, text: &str) -> Input {
    let format = format_of(name);
    Input::new_with_format(name, text, format)
}

#[test]
fn the_fixtures_place_every_path_as_recorded() {
    let mut lines = Vec::new();
    for file in fixtures() {
        let Some((name, text)) = named(&file) else {
            continue;
        };
        let format = format_of(&name);
        let Ok(tree) = format.read(&text, &Limits::NONE, Reading::Listing) else {
            let refused = input(&name, &text).place(&FieldPath::root()).is_none();
            lines.push(format!(
                "{name}: refused by the format, placed nowhere: {refused}"
            ));
            continue;
        };
        let input = input(&name, &text);
        let mut found = Vec::new();
        paths(&tree, &FieldPath::root(), &mut found);
        let at = |place: Option<ctl_input::Place>| {
            place.map_or_else(
                || "none".to_owned(),
                |at| format!("{}:{}", at.line, at.column),
            )
        };
        for path in found {
            lines.push(format!(
                "{name} {path:?}: value {}, key {}",
                at(input.place(&path)),
                at(input.place_key(&path))
            ));
        }
    }
    assert!(lines.len() > 500, "{} paths", lines.len());
    insta::assert_snapshot!("places", lines.join("\n"));
}

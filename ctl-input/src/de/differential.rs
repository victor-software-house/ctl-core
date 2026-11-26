//! The Deserializer against `Input::parse`, for every fixture and for
//! the types the existing input tests read: the typeless `serde_json::Value`
//! and each typed struct they declare.
//!
//! A success must give an equal value. A failure must be a failure of both,
//! at the same line and column. The wording is not compared. Every
//! difference is one row of [`EXPLAINED`], naming the fixture, the type, and
//! what each reader gave, so a new difference in the same pair fails.

use std::collections::BTreeMap;
use std::fmt::{self, Debug};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::{DeserializeOwned, IgnoredAny};

use super::{Tagged, deserialize};
use crate::format::{self, Fault, Reading};
use crate::tree::differential::fixtures;
use crate::tree::{NodeKind, position};
use crate::{Format, Input, Limits};

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

/// `Tags` as `Input::parse` reads it, with the fork's `Tagged`.
mod fork {
    use serde::Deserialize;

    use super::Setting;

    #[derive(Debug, Deserialize, PartialEq)]
    pub(super) struct Tags {
        pub(super) styles: crate::Tagged<Setting>,
        pub(super) level: crate::Tagged<Setting>,
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct Tags {
    styles: Tagged<Setting>,
    level: Tagged<Setting>,
}

/// A typeless value, printed as compact JSON so a row names it briefly.
#[derive(Deserialize, PartialEq)]
#[serde(transparent)]
struct Json(serde_json::Value);

impl Debug for Json {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// What reading one fixture as one type gave, in a form two readers compare.
#[derive(Debug, PartialEq)]
enum Outcome {
    Value(String),
    Refused(Option<(usize, usize)>),
}

impl fmt::Display for Outcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(value) => write!(formatter, "value {value}"),
            Self::Refused(Some((line, column))) => write!(formatter, "refused {line}:{column}"),
            Self::Refused(None) => formatter.write_str("refused"),
        }
    }
}

/// What `Input::parse` gave, and the words of its first problem.
fn today<T: DeserializeOwned + Debug>(input: &Input) -> (Outcome, String) {
    match input.parse::<T>() {
        Ok(value) => (Outcome::Value(format!("{value:?}")), String::new()),
        Err(error) => (
            Outcome::Refused(
                error
                    .problems
                    .first()
                    .map(|problem| (problem.line.unwrap_or(0), problem.column.unwrap_or(0))),
            ),
            error
                .problems
                .first()
                .map_or_default(|problem| problem.message.clone()),
        ),
    }
}

/// What the Deserializer gave, and the fault when it refused.
fn through_the_tree<T: DeserializeOwned + Debug>(
    text: &str,
    format: &'static dyn format::Format,
) -> (Outcome, Option<Fault>) {
    match format
        .read(text, &Limits::NONE, Reading::Strict)
        .and_then(|tree| deserialize::<T>(&tree, format))
    {
        Ok(value) => (Outcome::Value(format!("{value:?}")), None),
        Err(refusal) => (
            Outcome::Refused(refusal.span().map(|span| position(text, span.start))),
            Some(refusal.fault().clone()),
        ),
    }
}

/// One fixture read as one type, read differently by the two readers.
struct Difference {
    file: String,
    label: &'static str,
    before: Outcome,
    message: String,
    after: Outcome,
    fault: Option<Fault>,
    /// Where today's reader places a field the root mapping lacks: on the
    /// last key it read.
    root: (usize, usize),
}

impl Difference {
    /// A field the root mapping lacks, which today's reader places on the last
    /// key of the root and the tree on the start of the text, as the design
    /// rules. The field and today's place are checked, so no other
    /// difference passes as this one.
    fn is_root_missing_field(&self) -> bool {
        let Some(Fault::MissingField { field }) = &self.fault else {
            return false;
        };
        self.message == format!("missing field `{field}`")
            && self.before == Outcome::Refused(Some(self.root))
            && self.after == Outcome::Refused(Some((1, 1)))
    }
}

impl fmt::Display for Difference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} as {}: input {} ({}), tree {} ({:?})",
            self.file, self.label, self.before, self.message, self.after, self.fault
        )
    }
}

/// One fixture and type of the two readers compared, `Old` read by
/// `Input::parse` and `New` through the tree. Both print alike when they
/// hold the same value.
struct Compare<'a> {
    file: &'a str,
    input: &'a Input,
    format: &'static dyn format::Format,
    root: (usize, usize),
    differences: Vec<Difference>,
    compared: usize,
}

impl Compare<'_> {
    fn of<Old: DeserializeOwned + Debug, New: DeserializeOwned + Debug>(
        &mut self,
        label: &'static str,
    ) {
        let (before, message) = today::<Old>(self.input);
        let (after, fault) = through_the_tree::<New>(self.input.text(), self.format);
        self.compared += 1;
        if before != after {
            self.differences.push(Difference {
                file: self.file.to_owned(),
                label,
                before,
                message,
                after,
                fault,
                root: self.root,
            });
        }
    }

    fn same<T: DeserializeOwned + Debug>(&mut self, label: &'static str) {
        self.of::<T, T>(label);
    }
}

/// A difference a rule of the design explains, by fixture and type, with what
/// each reader gave: `input` is `Input::parse`, and `tree` is the
/// Deserializer. A text the format refuses is read before any type is, so it
/// is compared as `IgnoredAny` and `Value` only. A field the root lacks moves
/// from the root mapping to the start of the text, and is no row.
struct Explained {
    file: &'static str,
    label: &'static str,
    input: &'static str,
    tree: &'static str,
    rule: &'static str,
}

/// An entry is read in the order written, where today's reader reads the
/// entries of a TOML table by key, so the first refusal is another entry's.
const ORDER: &str = "entries come in the order written, not by key";
/// A block scalar is placed at its indicator, as the tree places every node.
const BLOCK: &str =
    "a block scalar is refused at its indicator, where today's reader refuses it at its first line";
/// A TOML datetime is its text, which fills a string, and a typeless read
/// gives it as that text, where the `toml` crate gives a private marker map.
const DATETIME: &str = "a datetime is its text, where the toml crate's own reader gives a private marker map and refuses it for a string";
/// A TOML datetime fills a string, and the entries come in the order written.
const DATETIME_ORDER: &str = "a datetime fills a string, so the first refusal is a later entry's";
/// A value the type ignores is not read, so a float it cannot type is no
/// error, and a field the root lacks is found first.
const IGNORED: &str = "an ignored value is not read, so a float it cannot type is no error";
/// A field a replayed mapping lacks sits on its key, where today's reader
/// places it on the alias that replayed it.
const REPLAYED: &str = "a missing field of a replayed mapping sits on its key, not on the alias";

const EXPLAINED: &[Explained] = &[
    Explained {
        file: "block-scalars.yaml",
        label: "Flags",
        input: "refused 2:3",
        tree: "refused 1:10",
        rule: BLOCK,
    },
    Explained {
        file: "block-scalars.yaml",
        label: "Ports",
        input: "refused 2:3",
        tree: "refused 1:10",
        rule: BLOCK,
    },
    Explained {
        file: "dotted-inline.toml",
        label: "Value",
        input: r#"value {"a":{"b":{"c":1}},"literal key":"w","point":{"x":1,"y":2},"products":[{"name":"a"},{"name":"b","tags":["x","y"]}],"quoted.key":"v","server":{"port":8080},"when":{"$__toml_private_datetime":"1979-05-27T07:32:00Z"}}"#,
        tree: r#"value {"a":{"b":{"c":1}},"literal key":"w","point":{"x":1,"y":2},"products":[{"name":"a"},{"name":"b","tags":["x","y"]}],"quoted.key":"v","server":{"port":8080},"when":"1979-05-27T07:32:00Z"}"#,
        rule: DATETIME,
    },
    Explained {
        file: "dotted-inline.toml",
        label: "Dated",
        input: "refused 5:8",
        tree: r#"value Dated { when: "1979-05-27T07:32:00Z", date: "" }"#,
        rule: DATETIME,
    },
    Explained {
        file: "input-16.yaml",
        label: "Nested",
        input: "refused 3:7",
        tree: "refused 3:1",
        rule: REPLAYED,
    },
    Explained {
        file: "input-21.yaml",
        label: "IgnoredAny",
        input: "refused 1:5",
        tree: "value IgnoredAny",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Ledger",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Port",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Moded",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Copied",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Nested",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Globs",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Shipped",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Tags",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Config",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "TomlConfig",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-21.yaml",
        label: "Dated",
        input: "refused 1:5",
        tree: r#"value Dated { when: "", date: "" }"#,
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "IgnoredAny",
        input: "refused 1:5",
        tree: "value IgnoredAny",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Ledger",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Port",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Moded",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Copied",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Nested",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Globs",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Shipped",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Tags",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Config",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "TomlConfig",
        input: "refused 1:5",
        tree: "refused 1:1",
        rule: IGNORED,
    },
    Explained {
        file: "input-34.yaml",
        label: "Dated",
        input: "refused 1:5",
        tree: r#"value Dated { when: "", date: "" }"#,
        rule: IGNORED,
    },
    Explained {
        file: "input_toml-01.toml",
        label: "Flags",
        input: "refused 2:10",
        tree: "refused 1:8",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-01.toml",
        label: "Ports",
        input: "refused 2:10",
        tree: "refused 1:8",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-02.toml",
        label: "Flags",
        input: "refused 2:10",
        tree: "refused 1:8",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-02.toml",
        label: "Ports",
        input: "refused 2:10",
        tree: "refused 1:8",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-03.toml",
        label: "Flags",
        input: "refused 2:10",
        tree: "refused 1:8",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-03.toml",
        label: "Ports",
        input: "refused 2:10",
        tree: "refused 1:8",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-04.toml",
        label: "Flags",
        input: "refused 2:10",
        tree: "refused 1:8",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-04.toml",
        label: "Ports",
        input: "refused 2:10",
        tree: "refused 1:8",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-13.toml",
        label: "Value",
        input: r#"value {"date":{"$__toml_private_datetime":"2001-01-01"},"list":[1,2],"literal":"x","multi":"a\nb","number":12,"quoted":"double","table":{"inner":"x"},"title":"Basic"}"#,
        tree: r#"value {"date":"2001-01-01","list":[1,2],"literal":"x","multi":"a\nb","number":12,"quoted":"double","table":{"inner":"x"},"title":"Basic"}"#,
        rule: DATETIME,
    },
    Explained {
        file: "input_toml-13.toml",
        label: "Strict",
        input: "refused 8:1",
        tree: "refused 1:1",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-13.toml",
        label: "Flags",
        input: "refused 8:8",
        tree: "refused 1:9",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-13.toml",
        label: "Ports",
        input: "refused 8:8",
        tree: "refused 1:9",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-13.toml",
        label: "Names",
        input: "refused 8:8",
        tree: "refused 7:10",
        rule: ORDER,
    },
    Explained {
        file: "input_toml-13.toml",
        label: "Anythings",
        input: "refused 8:8",
        tree: "refused 9:8",
        rule: DATETIME_ORDER,
    },
    Explained {
        file: "input_toml-13.toml",
        label: "Dated",
        input: "refused 8:8",
        tree: r#"value Dated { when: "", date: "2001-01-01" }"#,
        rule: DATETIME,
    },
    Explained {
        file: "numbers.toml",
        label: "Strict",
        input: "refused 3:1",
        tree: "refused 1:1",
        rule: ORDER,
    },
    Explained {
        file: "numbers.toml",
        label: "Flags",
        input: "refused 3:10",
        tree: "refused 1:7",
        rule: ORDER,
    },
    Explained {
        file: "numbers.toml",
        label: "Ports",
        input: "refused 9:8",
        tree: "refused 5:7",
        rule: ORDER,
    },
    Explained {
        file: "numbers.toml",
        label: "Names",
        input: "refused 3:10",
        tree: "refused 1:7",
        rule: ORDER,
    },
];

#[test]
fn the_deserializer_reads_every_fixture_as_input_does() {
    let mut differences = Vec::new();
    let (mut compared, mut unread, mut reads) = (0, 0, 0);
    for file in fixtures() {
        let Some(stem) = file
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
        else {
            continue;
        };
        let text = fs::read_to_string(&file).unwrap();
        let (format, tree_format): (Format, &'static dyn format::Format) = match Path::new(&stem)
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some("yaml") => (Format::Yaml, &format::YAML),
            Some("toml") => (Format::Toml, &format::TOML),
            _ => continue,
        };
        let input = Input::new_with_format(stem.clone(), text, format);
        let read = tree_format.read(input.text(), &Limits::NONE, Reading::Strict);
        let root = read.as_ref().map_or((0, 0), |tree| {
            let last_key = match tree.kind() {
                NodeKind::Mapping(entries) => entries.last().map(|(key, _)| key.span().start),
                NodeKind::Sequence(_) | NodeKind::Scalar(_) => None,
            };
            position(input.text(), last_key.unwrap_or_else(|| tree.span().start))
        });
        let mut compare = Compare {
            file: &stem,
            input: &input,
            format: tree_format,
            root,
            differences: Vec::new(),
            compared: 0,
        };
        compare.same::<IgnoredAny>("IgnoredAny");
        compare.same::<Json>("Value");
        if read.is_err() {
            unread += 1;
        } else {
            compared += 1;
            compare.same::<Ledger>("Ledger");
            compare.same::<Port>("Port");
            compare.same::<Strict>("Strict");
            compare.same::<Moded>("Moded");
            compare.same::<Copied>("Copied");
            compare.same::<Nested>("Nested");
            compare.same::<Flags>("Flags");
            compare.same::<Ports>("Ports");
            compare.same::<Names>("Names");
            compare.same::<Anythings>("Anythings");
            compare.same::<Globs>("Globs");
            compare.same::<Shipped>("Shipped");
            compare.of::<fork::Tags, Tags>("Tags");
            compare.same::<Config>("Config");
            compare.same::<TomlConfig>("TomlConfig");
            compare.same::<Dated>("Dated");
        }
        reads += compare.compared;
        differences.append(&mut compare.differences);
    }
    let root_missing = differences
        .iter()
        .filter(|found| found.is_root_missing_field())
        .count();
    println!(
        "{compared} fixtures read and {unread} refused by the read, {reads} readings compared, {} differences: {root_missing} a missing field of the root, {} explained",
        differences.len(),
        differences.len() - root_missing
    );
    let mut unexplained: Vec<String> = Vec::new();
    let mut used = vec![false; EXPLAINED.len()];
    for found in &differences {
        if found.is_root_missing_field() {
            continue;
        }
        let rule = EXPLAINED.iter().position(|row| {
            row.file == found.file
                && row.label == found.label
                && row.input == found.before.to_string()
                && row.tree == found.after.to_string()
        });
        match rule {
            Some(index) => used[index] = true,
            None => unexplained.push(format!(
                "{found}\n    row: ({:?}, {:?}, {:?}, {:?})",
                found.file,
                found.label,
                found.before.to_string(),
                found.after.to_string()
            )),
        }
    }
    assert!(compared > 40, "{compared} fixtures read");
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
    let gone: Vec<_> = EXPLAINED
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|(row, _)| format!("{} as {}: {}", row.file, row.label, row.rule))
        .collect();
    assert!(
        gone.is_empty(),
        "an explained difference is gone:\n{}",
        gone.join("\n")
    );
}

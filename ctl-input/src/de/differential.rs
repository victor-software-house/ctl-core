//! The Deserializer against `Input::parse`, for every fixture and for
//! the types the existing input tests read: the typeless `serde_json::Value`
//! and each typed struct they declare.
//!
//! A success must give an equal value. A failure must be a failure of both,
//! at the same line and column. The wording is not compared.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::{DeserializeOwned, IgnoredAny};

use super::{Tagged, deserialize};
use crate::format::{self, Reading, Refusal};
use crate::tree::differential::fixtures;
use crate::tree::position;
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

/// What reading one fixture as one type gave, in a form two readers compare.
#[derive(Debug, PartialEq)]
enum Outcome {
    Value(String),
    Refused(Option<(usize, usize)>),
}

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

fn through_the_tree<T: DeserializeOwned + Debug>(
    text: &str,
    format: &'static dyn format::Format,
) -> (Outcome, String) {
    let refused = |refusal: Refusal| {
        let fault = format!(
            "{:?} at {:?}",
            refusal.fault(),
            refusal.span().and_then(|span| text
                .get(span)
                .map(|slice| slice.chars().take(24).collect::<String>()))
        );
        (
            Outcome::Refused(refusal.span().map(|span| position(text, span.start))),
            fault,
        )
    };
    match format
        .read(text, &Limits::NONE, Reading::Strict)
        .and_then(|tree| deserialize::<T>(&tree, format))
    {
        Ok(value) => (Outcome::Value(format!("{value:?}")), String::new()),
        Err(refusal) => refused(refusal),
    }
}

/// One line for each fixture and type the two readers read differently:
/// `Old` is read by `Input::parse`, and `New` through the tree. Both print
/// alike when they hold the same value.
fn compare<Old: DeserializeOwned + Debug, New: DeserializeOwned + Debug>(
    differences: &mut Vec<Difference>,
    file: &str,
    label: &'static str,
    input: &Input,
    format: &'static dyn format::Format,
) {
    let (before, message) = today::<Old>(input);
    let (after, fault) = through_the_tree::<New>(input.text(), format);
    if before != after {
        differences.push(Difference {
            file: file.to_owned(),
            label,
            line: format!(
                "{file} as {label}: input {before:?} ({message}), tree {after:?} ({fault})"
            ),
            missing_field: fault.starts_with("MissingField")
                && message.starts_with("missing field"),
            root: after == Outcome::Refused(Some((1, 1))),
        });
    }
}

/// One fixture read as one type, read differently by the two readers.
struct Difference {
    file: String,
    label: &'static str,
    line: String,
    missing_field: bool,
    root: bool,
}

/// What a rule of the design explains, by fixture and type, with the rule.
/// An empty type stands for every type. A text the format refuses is read
/// before any type is, so it is compared as `IgnoredAny` only. A missing field
/// moves to the line of the root, or to its key, and every one of these
/// fixtures lacks it in the root.
const EXPLAINED: &[(&str, &str, &str)] = &[
    (
        "block-scalars.yaml",
        "Flags",
        "a block scalar's span starts at its indicator, as today's place puts it, and today's parse error at its first line",
    ),
    (
        "dotted-inline.toml",
        "Value",
        "a datetime is its text, where the toml crate's own reader gives a private marker map",
    ),
    (
        "input_toml-13.toml",
        "Value",
        "a datetime is its text, where the toml crate's own reader gives a private marker map",
    ),
    (
        "input-21.yaml",
        "",
        "an ignored value is not read, so a float it cannot type is no error",
    ),
    (
        "input-34.yaml",
        "",
        "an ignored value is not read, so a float it cannot type is no error",
    ),
    (
        "input_toml-01.toml",
        "Flags",
        "entries come in the order written, not by key",
    ),
    (
        "input_toml-02.toml",
        "Flags",
        "entries come in the order written, not by key",
    ),
    (
        "input_toml-03.toml",
        "Flags",
        "entries come in the order written, not by key",
    ),
    (
        "input_toml-04.toml",
        "Flags",
        "entries come in the order written, not by key",
    ),
    (
        "input_toml-13.toml",
        "Strict",
        "entries come in the order written, not by key",
    ),
    (
        "input_toml-13.toml",
        "Flags",
        "entries come in the order written, not by key",
    ),
    (
        "numbers.toml",
        "Strict",
        "entries come in the order written, not by key",
    ),
    (
        "numbers.toml",
        "Flags",
        "entries come in the order written, not by key",
    ),
];

#[test]
fn the_deserializer_reads_every_fixture_as_input_does() {
    let mut differences = Vec::new();
    let (mut compared, mut unread) = (0, 0);
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
        let d = &mut differences;
        compare::<IgnoredAny, IgnoredAny>(d, &stem, "IgnoredAny", &input, tree_format);
        if tree_format
            .read(input.text(), &Limits::NONE, Reading::Strict)
            .is_err()
        {
            unread += 1;
            continue;
        }
        compared += 1;
        compare::<serde_json::Value, serde_json::Value>(d, &stem, "Value", &input, tree_format);
        compare::<Ledger, Ledger>(d, &stem, "Ledger", &input, tree_format);
        compare::<Port, Port>(d, &stem, "Port", &input, tree_format);
        compare::<Strict, Strict>(d, &stem, "Strict", &input, tree_format);
        compare::<Moded, Moded>(d, &stem, "Moded", &input, tree_format);
        compare::<Copied, Copied>(d, &stem, "Copied", &input, tree_format);
        compare::<Flags, Flags>(d, &stem, "Flags", &input, tree_format);
        compare::<Globs, Globs>(d, &stem, "Globs", &input, tree_format);
        compare::<Shipped, Shipped>(d, &stem, "Shipped", &input, tree_format);
        compare::<fork::Tags, Tags>(d, &stem, "Tags", &input, tree_format);
        compare::<Config, Config>(d, &stem, "Config", &input, tree_format);
        compare::<TomlConfig, TomlConfig>(d, &stem, "TomlConfig", &input, tree_format);
    }
    let missing = differences
        .iter()
        .filter(|found| found.missing_field)
        .count();
    println!(
        "{compared} fixtures read, {unread} refused by the read, {} differences, {missing} of them a missing field",
        differences.len()
    );
    let mut unexplained: Vec<&str> = Vec::new();
    let mut used = vec![false; EXPLAINED.len()];
    for found in &differences {
        if found.missing_field && found.root {
            continue;
        }
        let rule = EXPLAINED.iter().position(|(file, label, _)| {
            *file == found.file && (label.is_empty() || *label == found.label)
        });
        match rule {
            Some(index) => used[index] = true,
            None => unexplained.push(&found.line),
        }
    }
    assert!(compared > 40, "{compared} fixtures read");
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
    assert!(
        used.iter().all(|used| *used),
        "an explained difference is gone: {used:?}"
    );
}

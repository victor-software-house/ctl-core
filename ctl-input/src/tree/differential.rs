//! Placing through the tree against placing today, for every path of every
//! fixture: the next change moves `Input` onto the tree, and each place it
//! reports must stay where it is.
//!
//! The paths are every node of the tree, and each of them with one missing key
//! appended. A node an alias replayed, and every path through one, is
//! skipped: today's placing sits such a problem on the anchor, and the next
//! change moves it to the alias. A key that is not a scalar has no step to
//! walk through, so each such key is counted and its value is not walked.
//!
//! For TOML this compares lookup and order only: `Input::place` and the tree
//! read the spans of the same `toml` parse, so a span cannot differ. YAML is
//! the comparison of two readers, serde-saphyr's and yamled's.

use std::fs;
use std::path::{Path, PathBuf};

use crate::format::Reading;
use crate::tree::{Node, NodeKind, position};
use crate::{At, FieldPath, Format, Formats, Input, Limits, Step};

/// What one walk of a tree found.
#[derive(Default)]
struct Walk {
    paths: Vec<Vec<Step>>,
    skipped: usize,
    non_scalar_keys: usize,
}

impl Walk {
    fn node(&mut self, node: &Node, steps: &[Step]) {
        if node.used_at().is_some() {
            self.skipped += 1;
            return;
        }
        self.paths.push(steps.to_vec());
        let mut missing = steps.to_vec();
        missing.push(Step::Key("no such key".to_owned()));
        self.paths.push(missing);
        match node.kind() {
            NodeKind::Scalar(_) => {}
            NodeKind::Sequence(items) => {
                for (index, item) in items.iter().enumerate() {
                    self.node(item, &[steps, &[Step::Index(index)]].concat());
                }
            }
            NodeKind::Mapping(entries) => {
                for (key, value) in entries {
                    if key.used_at().is_some() {
                        self.skipped += 1;
                    } else if let NodeKind::Scalar(scalar) = key.kind() {
                        let step = Step::Key(scalar.value().to_owned());
                        self.node(value, &[steps, &[step]].concat());
                    } else {
                        self.non_scalar_keys += 1;
                    }
                }
            }
        }
    }
}

fn field_path(steps: &[Step]) -> FieldPath {
    steps
        .iter()
        .fold(FieldPath::root(), |path, step| match step {
            Step::Key(key) => path.key(key.as_str()),
            Step::Index(index) => path.index(*index),
        })
}

/// The fixtures, each a text of one format whose extension comes before
/// `.fixture`, so the repository's formatters leave texts that are meant to
/// be wrong alone.
pub(crate) fn fixtures() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    files
}

/// The fixture as an input, when its format's feature is on.
fn input(file: &Path) -> Option<Input> {
    let name = file.file_stem()?.to_string_lossy().into_owned();
    let text = fs::read_to_string(file).unwrap();
    match Path::new(&name).extension()?.to_str()? {
        #[cfg(feature = "yaml")]
        "yaml" => Some(Input::new_with_format(name, text, Format::Yaml)),
        #[cfg(feature = "toml")]
        "toml" => Some(Input::new_with_format(name, text, Format::Toml)),
        _ => None,
    }
}

/// For each fixture that has any, the replayed nodes the walk skips and the
/// non-scalar keys it does not walk. A fixture not listed has neither, so a
/// fixture that starts to skip more fails here instead of passing with less
/// compared.
const EXPECTED: &[(&str, usize, usize)] = &[
    ("anchors-tags.yaml", 1, 0),
    ("input-07.yaml", 1, 0),
    ("input-16.yaml", 1, 0),
    ("input-19.yaml", 1, 0),
    ("input-24.yaml", 0, 2),
    ("input-25.yaml", 1, 0),
    ("input-36.yaml", 1, 0),
];

#[test]
fn the_tree_places_every_path_as_input_does() {
    let mut differences = Vec::new();
    let (mut files, mut texts, mut compared, mut skipped) = (0, 0, 0, 0);
    for file in fixtures() {
        let Some(input) = input(&file) else {
            continue;
        };
        let format = Formats::builtin().of(Path::new(input.name())).unwrap();
        files += 1;
        let name = input.name().to_owned();
        let tree = format.read(input.text(), &Limits::NONE, Reading::Listing);
        let Ok(tree) = tree else {
            texts += 1;
            if input.place(&FieldPath::root()).is_some() {
                differences.push(format!("{name}: placed a text the format refuses"));
            }
            continue;
        };
        let mut walk = Walk::default();
        walk.node(&tree, &[]);
        skipped += walk.skipped;
        let expected = EXPECTED
            .iter()
            .find(|(fixture, ..)| *fixture == name)
            .map_or((0, 0), |&(_, skips, keys)| (skips, keys));
        if (walk.skipped, walk.non_scalar_keys) != expected {
            differences.push(format!(
                "{name}: skipped {} replayed nodes and {} non-scalar keys, expected {expected:?}",
                walk.skipped, walk.non_scalar_keys
            ));
        }
        for steps in &walk.paths {
            let path = field_path(steps);
            for at in [At::Value, At::Key] {
                let found = position(input.text(), tree.nearest(steps, at).start);
                let expected = match at {
                    At::Value => input.place(&path),
                    At::Key => input.place_key(&path),
                }
                .map(|place| (place.line, place.column));
                compared += 1;
                if expected != Some(found) {
                    let at = match at {
                        At::Value => "value",
                        At::Key => "key",
                    };
                    differences.push(format!(
                        "{name}: {path} at {at}: input {expected:?}, tree {found:?}"
                    ));
                }
            }
        }
    }
    println!(
        "{files} fixtures, {texts} refused by the format, {compared} places compared, {skipped} replayed nodes skipped"
    );
    assert!(compared > 500, "{compared} places compared");
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

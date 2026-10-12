//! The Rust configuration every ctl repository shares, rendered from
//! templates embedded in this release.
//!
//! A repository declares what sets it apart in [`REPO_FILE`]. [`render`]
//! returns each served file whole: five files belong wholly to the baseline,
//! and in `Cargo.toml` and `mise.dev.toml` only the text between
//! `# ctl-core baseline: begin <block>` and `# ctl-core baseline: end <block>`
//! does, so the rest of those files stays the repository's. A runner writes
//! or compares the results; this module writes nothing.
//!
//! A repository changes the baseline in two ways only: it appends entries,
//! each with its reason, or it owns a whole file, which [`render`] then skips
//! and reports.

use std::path::Path;
use std::{fmt, fs};

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::input::{Input, InputError};
use crate::served::{self, Served};

/// Where a repository declares its baseline, relative to its root.
pub const REPO_FILE: &str = ".ctl/rust.yaml";

/// The marker that opens and closes a served block.
const MARKER: &str = "# ctl-core baseline:";

/// What a repository declares in [`REPO_FILE`]. Every field is optional.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// One crate, or a workspace. Unsaid means one crate.
    #[garde(skip)]
    pub layout: Layout,
    /// Whether the crate goes to `crates.io`, which adds cargo-semver-checks.
    #[garde(skip)]
    pub published: bool,
    /// Whether the crate uses tokio, which adds its lock guards and the
    /// unbounded-channel ban to `clippy.toml`.
    #[garde(skip)]
    pub tokio: bool,
    /// Entries appended to `clippy.toml`.
    #[garde(dive)]
    pub clippy: Clippy,
    /// Entries appended to `deny.toml`.
    #[garde(dive)]
    pub deny: Deny,
    /// Files the repository keeps for itself.
    #[garde(dive)]
    pub own: Vec<Owned>,
}

/// How the crates sit in the repository.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// One package: lints go under `[lints]`.
    #[default]
    Crate,
    /// A workspace: lints go under `[workspace.lints]`.
    Workspace,
}

/// Entries appended to `clippy.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields, default)]
pub struct Clippy {
    /// Added to `disallowed-methods`.
    #[garde(dive)]
    pub disallowed_methods: Vec<Disallowed>,
    /// Added to `disallowed-types`.
    #[garde(dive)]
    pub disallowed_types: Vec<Disallowed>,
}

/// A path clippy refuses, and why.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Disallowed {
    /// The method or type path.
    #[garde(custom(one_line))]
    pub path: String,
    /// Why, as clippy will print it.
    #[garde(custom(one_line))]
    pub reason: String,
}

/// Entries appended to `deny.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields, default)]
pub struct Deny {
    /// Licenses allowed beyond the baseline's.
    #[garde(dive)]
    pub licenses: Vec<License>,
    /// Crates banned beyond the baseline's.
    #[garde(dive)]
    pub bans: Vec<Ban>,
    /// Duplicate versions tolerated.
    #[garde(dive)]
    pub skip: Vec<Skip>,
    /// Advisories ignored.
    #[garde(dive)]
    pub ignore: Vec<Advisory>,
}

/// A license allowed, and which dependency needs it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct License {
    /// The SPDX identifier.
    #[garde(custom(one_line))]
    pub license: String,
    /// Which dependency needs it.
    #[garde(custom(one_line))]
    pub reason: String,
}

/// A crate banned, optionally except through named wrappers.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Ban {
    /// The crate name.
    #[serde(rename = "crate")]
    #[garde(custom(one_line))]
    pub name: String,
    /// The only crates that may depend on it directly. Unsaid means none.
    #[serde(default)]
    #[garde(inner(custom(one_line)))]
    pub wrappers: Vec<String>,
    /// Why.
    #[garde(custom(one_line))]
    pub reason: String,
}

/// A duplicate version tolerated, and when it can go.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Skip {
    /// The crate, optionally `name@version`.
    #[serde(rename = "crate")]
    #[garde(custom(one_line))]
    pub name: String,
    /// Why, and the condition for removing it.
    #[garde(custom(one_line))]
    pub reason: String,
}

/// An advisory ignored, and when it can go.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Advisory {
    /// The RUSTSEC id.
    #[garde(custom(one_line))]
    pub id: String,
    /// Why, and the condition for removing it.
    #[garde(custom(one_line))]
    pub reason: String,
}

/// A served file the repository keeps for itself.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Owned {
    /// The file.
    #[garde(skip)]
    pub file: File,
    /// Why the repository keeps it.
    #[garde(custom(one_line))]
    pub reason: String,
}

/// A file the baseline serves, whole or by one block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum File {
    /// The pinned stable, for editors and plain cargo.
    #[serde(rename = "rust-toolchain.toml")]
    RustToolchain,
    /// Formatting options.
    #[serde(rename = "rustfmt.toml")]
    Rustfmt,
    /// Clippy configuration.
    #[serde(rename = "clippy.toml")]
    Clippy,
    /// cargo-deny policy.
    #[serde(rename = "deny.toml")]
    Deny,
    /// Git hooks.
    #[serde(rename = "lefthook.yml")]
    Lefthook,
    /// The `lints` block.
    #[serde(rename = "Cargo.toml")]
    Cargo,
    /// The `tools` block.
    #[serde(rename = "mise.dev.toml")]
    MiseDev,
}

impl File {
    /// The file's name at the repository root.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::RustToolchain => "rust-toolchain.toml",
            Self::Rustfmt => "rustfmt.toml",
            Self::Clippy => "clippy.toml",
            Self::Deny => "deny.toml",
            Self::Lefthook => "lefthook.yml",
            Self::Cargo => "Cargo.toml",
            Self::MiseDev => "mise.dev.toml",
        }
    }

    /// The block this release serves inside the file, or `None` when it
    /// serves the whole file.
    const fn block(self) -> Option<&'static str> {
        match self {
            Self::Cargo => Some("lints"),
            Self::MiseDev => Some("tools"),
            Self::RustToolchain | Self::Rustfmt | Self::Clippy | Self::Deny | Self::Lefthook => {
                None
            }
        }
    }

    /// The embedded template.
    const fn template(self) -> &'static str {
        match self {
            Self::RustToolchain => include_str!("../templates/baseline/rust-toolchain.toml.jinja"),
            Self::Rustfmt => include_str!("../templates/baseline/rustfmt.toml.jinja"),
            Self::Clippy => include_str!("../templates/baseline/clippy.toml.jinja"),
            Self::Deny => include_str!("../templates/baseline/deny.toml.jinja"),
            Self::Lefthook => include_str!("../templates/baseline/lefthook.yml.jinja"),
            Self::Cargo => include_str!("../templates/baseline/Cargo.toml.jinja"),
            Self::MiseDev => include_str!("../templates/baseline/mise.dev.toml.jinja"),
        }
    }
}

/// Every file, in the order [`render`] returns them.
const FILES: [File; 7] = [
    File::RustToolchain,
    File::Rustfmt,
    File::Clippy,
    File::Deny,
    File::Lefthook,
    File::Cargo,
    File::MiseDev,
];

/// What [`render`] produced for a repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    /// Each served file, whole, ready to write or to compare.
    pub served: Vec<Served>,
    /// The files the repository keeps, with its reasons.
    pub owned: Vec<Owned>,
}

/// Why the baseline could not be rendered, naming the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineError {
    /// The file at the repository root.
    pub file: &'static str,
    /// What went wrong.
    pub message: String,
}

impl fmt::Display for BaselineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.file, self.message)
    }
}

impl std::error::Error for BaselineError {}

/// Read and validate `root`'s [`REPO_FILE`].
pub fn load(root: &Path) -> Result<Config, InputError> {
    Input::read(&root.join(REPO_FILE))?.load()
}

/// Render every file `config` does not own. A block file is read from
/// `root` and returned whole, with only its block replaced.
pub fn render(root: &Path, config: &Config) -> Result<Rendered, BaselineError> {
    let mut served = Vec::new();
    for file in FILES {
        if config.own.iter().any(|owned| owned.file == file) {
            continue;
        }
        let failed = |message: String| BaselineError {
            file: file.name(),
            message,
        };
        let template = format!("{}.jinja", file.name());
        let rendered = served::render(Path::new(&template), file.template(), ".jinja", config)
            .map_err(|error| failed(error.message))?;
        let rendered = match file.block() {
            None => rendered,
            Some(block) => {
                let current = fs::read_to_string(root.join(file.name()))
                    .map_err(|error| failed(error.to_string()))?;
                let body = splice(&current, block, rendered.body()).map_err(failed)?;
                rendered.with_body(body)
            }
        };
        served.push(rendered);
    }
    Ok(Rendered {
        served,
        owned: config.own.clone(),
    })
}

/// `current` with the text between `block`'s marker lines replaced by
/// `body`. A marker is a whole line, compared without trailing whitespace,
/// so a CRLF checkout still matches and `end lintsX` does not.
fn splice(current: &str, block: &str, body: &str) -> Result<String, String> {
    let begin = format!("{MARKER} begin {block}");
    let end = format!("{MARKER} end {block}");
    let mut start = None;
    let mut offset = 0;
    for line in current.split_inclusive('\n') {
        let text = line.trim_end();
        if let Some(from) = start {
            if text == end {
                return Ok(format!("{}{body}{}", &current[..from], &current[offset..]));
            }
        } else if text == begin {
            start = Some(offset + line.len());
        }
        offset += line.len();
    }
    Err(if start.is_some() {
        format!("has no `{end}` line after `{begin}`")
    } else {
        format!("has no `{begin}` line")
    })
}

/// A value that sits inside a TOML string or comment on one line.
fn one_line<C>(value: &str, _: &C) -> garde::Result {
    if value.is_empty() {
        return Err(garde::Error::new("cannot be empty"));
    }
    if value
        .chars()
        .any(|character| character.is_control() || character == '"' || character == '\\')
    {
        return Err(garde::Error::new(
            "must be one line without quotes or backslashes, so it fits a TOML string",
        ));
    }
    Ok(())
}

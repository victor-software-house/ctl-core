//! Files a repository serves, rendered from templates that declare where they
//! go.
//!
//! A template declares its target in Jinja's own syntax, as top-level
//! `{%- set -%}` exports, instead of a manifest beside it or a mirrored
//! directory tree:
//!
//! ```jinja
//! {%- set path = "tasks/q" -%}
//! {%- set executable = true -%}
//! #!/usr/bin/env bash
//! #MISE tools = { "github:victor-software-house/qctl" = "{{ versions["qctl"] }}" }
//! ```
//!
//! `path` is the directory, the repository root when unsaid. `name` is the
//! file name, the template's own name without its suffix when unsaid, so most
//! templates declare one line. `executable` is the one mode bit git records
//! for a regular file: unsaid means 0644. Trim declarations with `{%- … -%}`
//! so a shebang stays on line one. Any other export is the template's own.
//!
//! Rendering uses [`crate::surface::environment`]: undefined names fail, the
//! trailing newline is kept, and ctl-core's operator fragments are available.
//! Which templates a repository has, and what context they see, stay with the
//! CLI that serves them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fmt, fs};

use garde::Validate;
use minijinja::value::Value;
use minijinja::{Environment, State};
use serde::{Deserialize, Serialize};

use crate::validate::{inside_the_repo, one_file_name};

/// What a template declares about the file it serves, read as one schema
/// from its exports and checked once.
#[derive(Debug, Deserialize, Validate)]
#[garde(allow_unvalidated)]
struct Declared {
    #[serde(default)]
    #[garde(custom(inside_the_repo))]
    path: PathBuf,
    /// Seeded with the template's own name before parsing, so never absent.
    #[garde(custom(one_file_name))]
    name: String,
    #[serde(default)]
    executable: bool,
}

/// A rendered template and the file it serves, checked and not yet written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Served {
    target: PathBuf,
    body: String,
    executable: bool,
}

/// Why a template could not be served, naming the template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServeError {
    /// The template, as the caller named it.
    pub template: String,
    /// What went wrong.
    pub message: String,
}

impl fmt::Display for ServeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.template, self.message)
    }
}

impl std::error::Error for ServeError {}

/// Render one template with `context` and read the file it declares.
///
/// `template` names the template, usually its path relative to the
/// repository; its file name minus `suffix` is the default target name.
///
/// # Errors
///
/// When the file name does not end in `suffix`, the template does not parse
/// or render, or its declarations do not fit the schema: a `path` outside the
/// repository, a `name` that is not one file name, or a value of the wrong
/// type.
pub fn render(
    template: &Path,
    source: &str,
    suffix: &str,
    context: &impl Serialize,
) -> Result<Served, ServeError> {
    let failed = |message: String| ServeError {
        template: template.display().to_string(),
        message,
    };
    let own_name = template
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(suffix))
        .filter(|name| !name.is_empty())
        .ok_or_else(|| failed(format!("template name does not end in {suffix}")))?;
    let environment: Environment<'static> = crate::surface::environment()
        .map_err(|error| failed(format!("register ctl-core fragments: {error}")))?;
    let name = template.display().to_string();
    let compiled = environment
        .template_from_named_str(&name, source)
        .map_err(|error| failed(format!("parse: {error}")))?;
    let captured = compiled
        .render_captured(Value::from_serialize(context))
        .map_err(|error| failed(format!("render: {error}")))?;
    let declared = declarations(captured.state(), own_name)
        .map_err(|message| failed(format!("declarations: {message}")))?;
    Ok(Served {
        target: declared.path.join(&declared.name),
        body: captured.output().to_owned(),
        executable: declared.executable,
    })
}

/// The schema read out of an evaluated template at once, over a seeded
/// `name`. Undefined and `none` exports count as unsaid.
fn declarations(state: &State<'_, '_>, own_name: &str) -> Result<Declared, String> {
    let mut exports: BTreeMap<&str, Value> = state
        .exports()
        .into_iter()
        .filter_map(|name| state.lookup(name).map(|value| (name, value)))
        .filter(|(_, value)| !value.is_undefined() && !value.is_none())
        .collect();
    exports
        .entry("name")
        .or_insert_with(|| Value::from(own_name));
    let declared = Declared::deserialize(Value::from_serialize(&exports))
        .map_err(|error| error.to_string())?;
    declared.validate().map_err(|report| report.to_string())?;
    Ok(declared)
}

impl Served {
    /// Where the file goes, relative to the repository root.
    #[must_use]
    pub fn target(&self) -> &Path {
        &self.target
    }

    /// The rendered text.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Whether the file is written 0755 rather than 0644.
    #[must_use]
    pub fn executable(&self) -> bool {
        self.executable
    }

    /// Write the file under `root`, creating its directories, and set its mode
    /// on Unix. Returns the full path.
    ///
    /// # Errors
    ///
    /// When the target is a symlink, which could point outside the
    /// repository, or the file or its directories cannot be written.
    pub fn write(&self, root: &Path) -> Result<PathBuf, ServeError> {
        let failed = |message: String| ServeError {
            template: self.target.display().to_string(),
            message,
        };
        let path = root.join(&self.target);
        if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(failed("target must not be a symlink".to_owned()));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| failed(format!("create {}: {error}", parent.display())))?;
        }
        fs::write(&path, &self.body).map_err(|error| failed(format!("write: {error}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if self.executable { 0o755 } else { 0o644 };
            fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                .map_err(|error| failed(format!("chmod: {error}")))?;
        }
        Ok(path)
    }
}

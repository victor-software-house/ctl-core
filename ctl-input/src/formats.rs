//! The registry of input formats, and the process default an input takes it
//! from.

use std::fmt;
use std::path::Path;
use std::sync::{PoisonError, RwLock};

use super::format::Format;
use super::{Input, InputError, read_text};

/// The input formats a CLI reads, in the order they were registered.
///
/// [`Self::builtin`] holds `yaml` and then `toml`, each with its feature. A
/// CLI adds a format, replaces a built-in of the same name, or removes one,
/// then installs the set with [`set_default_formats`]. Every builder takes and
/// returns `self`, as ctl-core's output registry does.
///
/// ```
/// # #[cfg(all(feature = "yaml", feature = "toml"))]
/// # {
/// use ctl_input::Formats;
///
/// let formats = Formats::builtin().remove("toml");
/// assert_eq!(formats.names().collect::<Vec<_>>(), ["yaml"]);
/// # }
/// ```
#[derive(Clone)]
pub struct Formats {
    registered: Vec<&'static dyn Format>,
}

impl Formats {
    /// `yaml` with the `yaml` feature, then `toml` with the `toml` feature.
    #[must_use]
    pub fn builtin() -> Self {
        let formats = Self::empty();
        #[cfg(feature = "yaml")]
        let formats = formats.add(&super::format::YAML);
        #[cfg(feature = "toml")]
        let formats = formats.add(&super::format::TOML);
        formats
    }

    /// No format: [`Self::of`] refuses every extension.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            registered: Vec::new(),
        }
    }

    /// Register `format`, or replace the registered format of the same name.
    /// A replacement keeps its place in the list.
    #[must_use]
    #[expect(
        clippy::should_implement_trait,
        reason = "a builder named for what it does, as `remove` is; `Add` has no meaning here"
    )]
    pub fn add(mut self, format: &'static dyn Format) -> Self {
        let name = format.name();
        match self.registered.iter_mut().find(|held| held.name() == name) {
            Some(held) => *held = format,
            None => self.registered.push(format),
        }
        self
    }

    /// Take the format called `name` out.
    #[must_use]
    pub fn remove(mut self, name: &str) -> Self {
        self.registered.retain(|held| held.name() != name);
        self
    }

    /// The registered format called `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&'static dyn Format> {
        self.registered
            .iter()
            .copied()
            .find(|format| format.name() == name)
    }

    /// The registered names, in the order registered.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.registered.iter().map(|format| format.name())
    }

    /// The registered format that reads a file's extension, compared without
    /// regard to case: with the built-ins, `.toml` is TOML, and `.yaml` and
    /// `.yml` are YAML.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with no line for any other extension or none, naming
    /// the extension and the accepted ones. One naming the feature for a
    /// built-in extension whose feature is off.
    pub fn of(&self, path: &Path) -> Result<&'static dyn Format, InputError> {
        let name = path.display().to_string();
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
        let found = extension.as_deref().and_then(|extension| {
            self.registered
                .iter()
                .copied()
                .find(|format| format.extensions().contains(&extension))
        });
        if let Some(format) = found {
            return Ok(format);
        }
        let accepted = self.accepted();
        match extension.as_deref() {
            #[cfg(not(feature = "yaml"))]
            Some(yaml @ ("yaml" | "yml")) => Err(InputError::whole(
                name,
                format!("`.{yaml}` needs ctl-core's input-yaml feature; {accepted}"),
            )),
            #[cfg(not(feature = "toml"))]
            Some("toml") => Err(InputError::whole(
                name,
                format!("`.toml` needs ctl-core's input-toml feature; {accepted}"),
            )),
            Some(other) => Err(InputError::whole(
                name,
                format!("unsupported extension .{other}; {accepted}"),
            )),
            None => Err(InputError::whole(
                name,
                format!("no file extension; {accepted}"),
            )),
        }
    }

    /// Read a file with the format this registry has for its extension. The
    /// extension is checked before the file is read, so a file no registered
    /// format reads is refused without being opened.
    ///
    /// [`Input::open`] does the same through the registry the process uses.
    ///
    /// # Errors
    ///
    /// An [`InputError`] for an extension [`Self::of`] refuses, or with the I/O
    /// failure.
    pub fn open(&self, path: &Path) -> Result<Input, InputError> {
        let format = self.of(path)?;
        let (name, text) = read_text(path)?;
        Ok(Input::new_with_format(name, text, format))
    }

    /// What a refusal tells a person to name the file: every registered
    /// extension, sorted, as `*.ext`.
    fn accepted(&self) -> String {
        let mut extensions: Vec<String> = self
            .registered
            .iter()
            .flat_map(|format| format.extensions())
            .map(|extension| format!("*.{extension}"))
            .collect();
        extensions.sort();
        match extensions.as_slice() {
            [] => "no format is registered".to_owned(),
            [only] => format!("name the file {only}"),
            [first, second] => format!("name the file {first} or {second}"),
            [rest @ .., last] => format!("name the file {}, or {last}", rest.join(", ")),
        }
    }
}

/// Names the registered formats: a format has no `Debug`.
impl fmt::Debug for Formats {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Formats")
            .field("names", &self.names().collect::<Vec<_>>())
            .finish()
    }
}

static PROCESS: RwLock<Option<Formats>> = RwLock::new(None);

/// Set the process-wide registry, which [`Input::open`] reads through. Without
/// a call, the process uses [`Formats::builtin`]. [`Formats::open`] reads
/// through a registry of the caller's instead.
pub fn set_default_formats(formats: Formats) {
    *PROCESS.write().unwrap_or_else(PoisonError::into_inner) = Some(formats);
}

/// The registry [`Input::open`] reads through: the process default, or
/// [`Formats::builtin`].
pub(crate) fn default_formats() -> Formats {
    PROCESS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .unwrap_or_else(Formats::builtin)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The process default is global, so this is the one test that sets it.
    #[test]
    fn the_process_default_replaces_the_builtin_registry() {
        let builtin = default_formats().names().count();
        assert!(builtin > 0, "without a call the process uses the built-ins");
        set_default_formats(Formats::empty());
        assert_eq!(default_formats().names().count(), 0);
        set_default_formats(Formats::builtin());
        assert_eq!(default_formats().names().count(), builtin);
    }
}

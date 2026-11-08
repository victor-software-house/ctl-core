//! The shells a ctl CLI writes scripts for, and the script a shell verb
//! prints.
//!
//! [`Shell`] needs the `shell` feature. [`Script`] and [`view`] also need
//! `view`. The `completion` and `shell-init` features build the verbs that
//! return a [`Script`].

#[cfg(feature = "view")]
use serde::Serialize;

#[cfg(feature = "view")]
use crate::color::ColorMode;
#[cfg(feature = "view")]
use crate::document::Document;
#[cfg(feature = "view")]
use crate::format::OutputFormat;
#[cfg(feature = "view")]
use crate::view::{Present, View};

/// A shell a ctl CLI prints a script for: the set Homebrew installs
/// completion for by default.
///
/// Every verb that takes a `Shell` supports every variant. A shell joins the
/// set only when each of those verbs has a tested form for it.
///
/// The set is closed, and the enum is deliberately not `#[non_exhaustive]`.
/// A consumer matches on every variant, so the compiler shows each place a
/// new shell needs a script. Adding a shell is therefore a breaking release.
///
/// The command-line value is the lowercase name: `bash`, `zsh`, or `fish`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
#[cfg_attr(feature = "json", serde(rename_all = "lowercase"))]
pub enum Shell {
    /// Bash.
    Bash,
    /// Zsh.
    Zsh,
    /// Fish.
    Fish,
}

/// One script for one shell, the result of a shell verb such as
/// `completion` or `init`.
///
/// Pretty and colorless output print the script as it is. JSON carries it
/// under `text`, beside the shell's name under `shell`.
#[cfg(feature = "view")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Script {
    /// The shell that reads the script.
    pub shell: Shell,
    /// The script.
    pub text: String,
}

#[cfg(feature = "view")]
impl Present for Script {
    fn present(&self) -> Document {
        Document::new().verbatim(self.text.clone())
    }
}

/// The view a shell verb prints its [`Script`] through: `format`, no colour,
/// and never quiet.
///
/// The script is the verb's whole result. `source <(toy completion zsh)` or
/// `eval "$(toy init zsh)"` would load nothing if `--quiet` or a quiet
/// variable hid it. Route the verb here from
/// [`App::view`](crate::app::App::view).
#[cfg(feature = "view")]
#[must_use]
pub const fn view(format: OutputFormat) -> View {
    View::new(format, ColorMode::Never)
}

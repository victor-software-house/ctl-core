//! The shells a ctl CLI writes scripts for.

/// A shell a ctl CLI prints a script for: the set Homebrew installs
/// completion for by default.
///
/// Every verb that takes a `Shell` supports every variant, so a shell joins
/// the set only when each of those verbs has a tested form for it.
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

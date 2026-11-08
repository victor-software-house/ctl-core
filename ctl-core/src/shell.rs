//! The shells a ctl CLI writes scripts for.

/// A shell a ctl CLI prints a script for: the set Homebrew installs
/// completion for by default.
///
/// Every verb that takes a `Shell` supports every variant, so a shell joins
/// the set only when each of those verbs has a tested form for it.
///
/// The set is closed, and the enum is deliberately not `#[non_exhaustive]`:
/// a consumer matches on every variant, so the compiler shows each place a
/// new shell needs a script. Adding a shell is therefore a breaking release.
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

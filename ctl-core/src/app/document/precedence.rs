//! Which document keys the command line or the environment already decide.

use clap::Arg;

use super::encode;
use crate::flags::Placed;

/// What a document key lost to.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Loss {
    /// An argument the command line set, named as it is written there.
    Line(String),
    /// A variable that stays bound, so it counts as set.
    Variable(String),
}

impl Loss {
    /// The words a problem names it by.
    pub(super) fn describe(&self) -> String {
        match self {
            Self::Line(flag) => format!("`{flag}`, which the command line sets"),
            Self::Variable(name) => format!("the variable `{name}`"),
        }
    }
}

/// The record of `arg` among `placed`: the one at the deepest level, else a
/// global that a level above declares.
/// `placed` lists a global only where it is declared.
pub(super) fn find<'p, 'a>(
    placed: &'p [Placed<'a>],
    arg: &Arg,
    deepest: usize,
) -> Option<&'p Placed<'a>> {
    let same = |entry: &&Placed<'_>| entry.arg.get_id() == arg.get_id();
    placed
        .iter()
        .filter(same)
        .find(|entry| entry.depth == deepest)
        .or_else(|| {
            placed
                .iter()
                .filter(same)
                .find(|entry| entry.arg.is_global_set())
        })
}

/// What the key for `key` lost to, if anything.
///
/// A key loses to the command line when the command line set the same
/// argument, the other side of its `--x`/`--no-x` pair, or an argument it
/// conflicts with.
/// It then loses to a variable that stays bound, in the same three cases.
///
/// The variable of an overruled option is unbound already.
/// So a bound variable that holds a value is a kept one.
pub(super) fn lost(key: &Placed<'_>, placed: &[Placed<'_>]) -> Option<Loss> {
    let named = |other: &Placed<'_>| Loss::Line(encode::name(other.arg));
    if key.given {
        return Some(named(key));
    }
    if let Some(other) = placed
        .iter()
        .find(|other| other.given && (key.negates(other) || key.conflicts(other)))
    {
        return Some(named(other));
    }
    let kept = |other: &Placed<'_>| other.arg.get_env().is_some() && other.is_set();
    if kept(key) {
        return Some(Loss::Variable(key.env_name()));
    }
    placed
        .iter()
        .find(|other| kept(other) && (key.negates(other) || key.conflicts(other)))
        .map(|other| Loss::Variable(other.env_name()))
}

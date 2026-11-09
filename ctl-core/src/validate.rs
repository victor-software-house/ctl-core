//! Garde rules every declared schema shares. A repository's config and a
//! template's own exports are held to the same rules and report the same
//! messages.
//!
//! Each one is a `#[garde(custom(…))]` rule. It runs at the boundary, on a
//! fully parsed value, and garde names the field that failed. The rules are
//! generic over the context. The same rule works in a schema validated
//! against nothing and in one validated against the whole document.
//!
//! ```
//! use ctl_core::validate::{at_least_one, one_file_name};
//!
//! assert!(one_file_name::<()>("notes.md", &()).is_ok());
//! assert!(one_file_name::<()>("docs/notes.md", &()).is_err());
//! assert!(at_least_one::<String, ()>("label")(&[], &()).is_err());
//! ```

use std::path::{Component, Path};

/// A path a repository may write to: relative, and never upward out of the
/// tree.
///
/// # Errors
///
/// "must stay inside the repository" for an absolute path or one with
/// `..`.
pub fn inside_the_repo<C>(path: &Path, _: &C) -> garde::Result {
    if path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(garde::Error::new("must stay inside the repository"));
    }
    Ok(())
}

/// A rule for a list that must not be empty. The `noun` names what the list
/// holds, in the message: `at_least_one("label")` reports "must declare at
/// least one label".
///
/// # Errors
///
/// The returned rule fails with that message when the list is empty.
pub fn at_least_one<T, C>(noun: &'static str) -> impl Fn(&[T], &C) -> garde::Result {
    move |items, _| {
        if items.is_empty() {
            return Err(garde::Error::new(format!(
                "must declare at least one {noun}"
            )));
        }
        Ok(())
    }
}

/// A rule for a string that must not be empty. The `because` text gives the
/// reason, so the message explains itself.
///
/// The message is "cannot be empty", a dash, and `because`.
///
/// # Errors
///
/// The returned rule fails with that message when the string is empty.
pub fn cannot_be_empty<C>(because: &'static str) -> impl Fn(&str, &C) -> garde::Result {
    move |value, _| {
        if value.is_empty() {
            return Err(garde::Error::new(format!("cannot be empty — {because}")));
        }
        Ok(())
    }
}

/// One file name, so a name cannot smuggle in the path it was separated from.
///
/// # Errors
///
/// "must be one file name" for a name with a directory part.
pub fn one_file_name<C>(name: &str, _: &C) -> garde::Result {
    let path = Path::new(name);
    if path.file_name().is_none_or(|only| only != name) {
        return Err(garde::Error::new("must be one file name"));
    }
    Ok(())
}

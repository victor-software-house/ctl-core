//! The steps from a document's root to a field, and the place where a field is
//! written.

use std::fmt;

#[cfg(feature = "validate")]
use garde::error;

/// One step down a [`FieldPath`]: a mapping key or a sequence index.
///
/// The set is closed, because a node of a YAML or TOML document is entered by
/// one of these two ways and by no other.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    /// A key of a mapping or table. Any text: it may hold `.`, `*`, `/`, `[`,
    /// or only digits.
    Key(String),
    /// An item of a sequence or array.
    Index(usize),
}

/// The steps from a document's root down to a field.
///
/// Build one with [`FieldPath::root`], [`FieldPath::key`], and
/// [`FieldPath::index`], or, with the `validate` feature, from garde's path
/// with `FieldPath::from`. [`Input::place`](super::Input::place) turns it into
/// a line and column.
///
/// A key is kept as written, so
/// `FieldPath::root().key("files").key("docs/**/*.md")` names one key of
/// `files`, not four. `Display` joins the steps the way garde prints a field,
/// `files.docs/**/*.md`, which cannot tell a key that holds a dot from nesting.
/// It is for people, and nothing reads it back.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FieldPath(Vec<Step>);

impl FieldPath {
    /// The document's root.
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// This path extended by a mapping or table key.
    #[must_use]
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.0.push(Step::Key(key.into()));
        self
    }

    /// This path extended by a sequence or array index.
    #[must_use]
    pub fn index(mut self, index: usize) -> Self {
        self.0.push(Step::Index(index));
        self
    }

    /// The steps from the root, in order.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.0
    }

    /// Whether this path's leading steps are the steps of `prefix`.
    #[cfg(feature = "validate")]
    pub(crate) fn starts_with(&self, prefix: &Self) -> bool {
        self.0.starts_with(&prefix.0)
    }

    /// This path without the steps at `positions`, each an index into
    /// [`Self::steps`].
    #[cfg(feature = "validate")]
    pub(crate) fn without(&self, positions: &[usize]) -> Self {
        Self(
            self.0
                .iter()
                .enumerate()
                .filter(|(position, _)| !positions.contains(position))
                .map(|(_, step)| step.clone())
                .collect(),
        )
    }
}

/// The path as garde prints a field: keys joined by `.`, indexes in brackets,
/// and nothing for the root.
impl fmt::Display for FieldPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (position, step) in self.0.iter().enumerate() {
            match step {
                Step::Key(key) => {
                    if position > 0 {
                        formatter.write_str(".")?;
                    }
                    formatter.write_str(key)?;
                }
                Step::Index(index) => write!(formatter, "[{index}]")?,
            }
        }
        Ok(())
    }
}

/// The steps of garde's path, read from its parts and never from its printed
/// string, so a key that holds a `.` stays one step.
///
/// A component garde marks as an index becomes [`Step::Index`], one it marks as
/// a key becomes [`Step::Key`], and one it marks with no key, which names a
/// wrapper such as a one-field tuple struct, is dropped. An index component
/// that is not a number is kept as a key.
#[cfg(feature = "validate")]
impl From<&error::Path> for FieldPath {
    fn from(path: &error::Path) -> Self {
        use garde::error::Kind;

        let mut steps = Vec::new();
        // garde 0.23.0 keeps a path's parts behind `__iter`, which is public
        // and `#[doc(hidden)]`. It yields the innermost component first, as
        // `Display` reads it with `rev`. The workspace pins garde exactly so a
        // release cannot change it unseen, and
        // `a_garde_path_converts_from_its_parts` fails first if one does.
        for (kind, component) in path.__iter().rev() {
            match kind {
                Kind::Key => steps.push(Step::Key(component.to_string())),
                Kind::Index => steps.push(
                    component
                        .parse::<usize>()
                        .map_or_else(|_| Step::Key(component.to_string()), Step::Index),
                ),
                Kind::None => {}
            }
        }
        Self(steps)
    }
}

/// The 1-based line and column where a field is written.
///
/// The column counts characters. [`Input::place`](super::Input::place) returns
/// one. The struct is non-exhaustive so a field such as an end position can
/// join it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Place {
    /// 1-based line in the whole file, frontmatter lines included.
    pub line: usize,
    /// 1-based column, counted in characters.
    pub column: usize,
}

impl Place {
    pub(crate) const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_prints_as_garde_prints_a_field() {
        let path = FieldPath::root()
            .key("files")
            .key("docs/**/*.md")
            .key("vocab")
            .index(1);
        assert_eq!(path.to_string(), "files.docs/**/*.md.vocab[1]");
        assert_eq!(FieldPath::root().index(0).key("a").to_string(), "[0].a");
        assert_eq!(FieldPath::root().to_string(), "");
    }

    #[cfg(feature = "validate")]
    #[test]
    fn a_garde_path_converts_from_its_parts() {
        let path = error::Path::new("files")
            .join("docs/**/*.md")
            .join("vocab")
            .join(0_usize);
        assert_eq!(
            FieldPath::from(&path).steps(),
            [
                Step::Key("files".to_owned()),
                Step::Key("docs/**/*.md".to_owned()),
                Step::Key("vocab".to_owned()),
                Step::Index(0),
            ]
        );
        assert_eq!(path.to_string(), "files.docs/**/*.md.vocab[0]");
    }

    #[cfg(feature = "validate")]
    #[test]
    fn a_numeric_key_stays_a_key_and_a_keyless_part_is_dropped() {
        let path = error::Path::new("files")
            .join("2")
            .join(error::NoKey::default())
            .join("label");
        assert_eq!(
            FieldPath::from(&path).steps(),
            [
                Step::Key("files".to_owned()),
                Step::Key("2".to_owned()),
                Step::Key("label".to_owned()),
            ]
        );
    }
}

//! Where a path is written in the text, and the garde check that places its
//! problems there.

use std::slice;

#[cfg(feature = "validate")]
use garde::Validate;

#[cfg(feature = "validate")]
use super::InputError;
use super::format::Reading;
use super::{FieldPath, Input, Place, Problem, limits, tree};

/// What a placing answers with for a key of the path: where its value starts,
/// or where the key itself is written.
#[derive(Clone, Copy)]
pub(crate) enum At {
    Value,
    Key,
}

impl Input {
    /// Declare that the schema field at `field` is flattened into its parent
    /// with `#[serde(flatten)]`, so the file writes its entries directly under
    /// the parent.
    ///
    /// garde names the field in each problem's path all the same, and serde
    /// gives no sign of the flattening at run time, so the schema declares it.
    /// [`Input::check`] then drops the field's step from each garde path whose
    /// leading steps are `field`, before it places the problem and prints its
    /// field. `field` is written in the schema's terms, the way garde names it,
    /// and each call adds one field. A declaration that matches no garde path
    /// changes nothing. [`Input::place`], [`Input::place_key`], and
    /// [`Input::problem_at`] take paths in the file's terms, so a declaration
    /// does not touch them.
    ///
    /// # Examples
    ///
    /// verctl's `assets` holds its targets in a flattened map field `targets`.
    /// Without the declaration, the problem prints as
    /// `assets.targets.linux-x64.files` and sits on the line below `assets:`.
    ///
    /// ```
    /// # #[cfg(all(feature = "yaml", feature = "validate"))]
    /// # {
    /// use std::collections::BTreeMap;
    ///
    /// use ctl_input::{FieldPath, Input};
    /// use garde::Validate;
    /// use serde::Deserialize;
    ///
    /// #[derive(Debug, Deserialize, Validate)]
    /// struct Config {
    ///     #[garde(dive)]
    ///     assets: Assets,
    /// }
    ///
    /// #[derive(Debug, Deserialize, Validate)]
    /// struct Assets {
    ///     #[serde(flatten)]
    ///     #[garde(dive)]
    ///     targets: BTreeMap<String, Target>,
    /// }
    ///
    /// #[derive(Debug, Deserialize, Validate)]
    /// struct Target {
    ///     #[garde(custom(at_least_one_file))]
    ///     files: Vec<String>,
    /// }
    ///
    /// fn at_least_one_file(files: &[String], _: &()) -> garde::Result {
    ///     if files.is_empty() {
    ///         return Err(garde::Error::new("must declare at least one file"));
    ///     }
    ///     Ok(())
    /// }
    ///
    /// let error = Input::new("ver.yaml", "assets:\n  linux-x64:\n    files: []\n")
    ///     .flattened(FieldPath::root().key("assets").key("targets"))
    ///     .load::<Config>()
    ///     .unwrap_err();
    /// assert_eq!(
    ///     error.to_string(),
    ///     "ver.yaml:3:12: assets.linux-x64.files: must declare at least one file"
    /// );
    /// # }
    /// ```
    #[cfg(feature = "validate")]
    #[must_use]
    pub fn flattened(mut self, field: FieldPath) -> Self {
        self.flattened.push(field);
        self
    }

    /// `path` without the step of each declared flattened field it passes
    /// through, or `None` when it passes through none.
    #[cfg(feature = "validate")]
    fn unflatten(&self, path: &FieldPath) -> Option<FieldPath> {
        let positions: Vec<usize> = self
            .flattened
            .iter()
            .filter(|field| path.starts_with(field))
            .filter_map(|field| field.steps().len().checked_sub(1))
            .collect();
        (!positions.is_empty()).then(|| path.without(&positions))
    }

    /// Validate a parsed `value` against `context`.
    ///
    /// Each of garde's problems lands on the line of the field it names, or
    /// of its nearest written ancestor. The place comes from the parts of
    /// garde's path through [`Input::place`], so a key that holds a `.` is
    /// found as one key. `Problem::field` is garde's printed path, for display
    /// only. A path through a field [`Input::flattened`] declares loses that
    /// field's step, and its `Problem::field` is the shorter path as
    /// [`FieldPath`] prints it.
    ///
    /// # Errors
    ///
    /// An [`InputError`] with every problem garde reports.
    #[cfg(feature = "validate")]
    pub fn check<T: Validate + ?Sized>(
        &self,
        value: &T,
        context: &T::Context,
    ) -> Result<(), InputError> {
        let Err(report) = value.validate_with(context) else {
            return Ok(());
        };
        let reported: Vec<_> = report
            .iter()
            .map(|(path, error)| {
                let steps = FieldPath::from(path);
                let (field, steps) = match self.unflatten(&steps) {
                    Some(kept) => (kept.to_string(), kept),
                    None => (path.to_string(), steps),
                };
                (field, steps, error.message().to_owned())
            })
            .collect();
        let paths: Vec<_> = reported.iter().map(|(_, path, _)| path.clone()).collect();
        let places = self.places(&paths, At::Value);
        let problems = reported
            .into_iter()
            .zip(places)
            .map(|((field, _, message), place)| Problem {
                line: place.map(|at| at.line),
                column: place.map(|at| at.column),
                field,
                message,
            });
        InputError::try_new(self.name.clone(), problems).map_or(Ok(()), Err)
    }

    /// Where `path` is written in the text, or where its nearest written
    /// ancestor is.
    ///
    /// The line counts the whole file, so a frontmatter input adds the lines
    /// before it. A path the text lacks gives the place of the last step that
    /// exists, and a caller that needs an exact hit compares the result with
    /// the place of the path's parent. `None` when the text does not parse.
    ///
    /// `None` too when the text goes past the limits in force, as
    /// [`Input::parse`] would refuse it, so placing never reads a text the
    /// limits keep out.
    ///
    /// The first placing reads the text into the tree and keeps it, so
    /// [`Input::parse`], [`Input::check`], [`Input::top_level`] and later calls
    /// share one read, for every format. A repeated key or a merge key does not
    /// stop placing, though [`Input::parse`] refuses it.
    ///
    /// ```
    /// # #[cfg(feature = "yaml")]
    /// # {
    /// use ctl_input::{FieldPath, Input};
    ///
    /// let input = Input::new("app.yaml", "files:\n  'docs/**/*.md':\n    label: ''\n");
    /// let path = FieldPath::root()
    ///     .key("files")
    ///     .key("docs/**/*.md")
    ///     .key("label");
    /// let at = input.place(&path).map(|at| (at.line, at.column));
    /// assert_eq!(at, Some((3, 12)));
    /// # }
    /// ```
    #[must_use]
    pub fn place(&self, path: &FieldPath) -> Option<Place> {
        self.places(slice::from_ref(path), At::Value)
            .pop()
            .flatten()
    }

    /// Where the last step of `path` is written as a key, or where its nearest
    /// written ancestor's key is.
    ///
    /// [`Input::place`] gives where a value starts. For a scalar or a flow
    /// value that is the line of the key, and for a block mapping or a block
    /// list it is the first line below it. A fault about the key itself, such
    /// as a key that names nothing, belongs on the key, and this finds it.
    ///
    /// A step that is an index has no key, and neither has the root, so they
    /// give the place of their value, as `place` does. A path the text lacks
    /// gives the key of the last step that exists. The line counts the whole
    /// file, `None` is for a text that does not parse or goes past the limits,
    /// and the tree is read once and shared with [`Input::place`].
    ///
    /// ```
    /// # #[cfg(feature = "yaml")]
    /// # {
    /// use ctl_input::{FieldPath, Input};
    ///
    /// let input = Input::new("app.yaml", "files:\n  'docs/**/*.md':\n    label: ''\n");
    /// let glob = FieldPath::root().key("files").key("docs/**/*.md");
    /// let value = input.place(&glob).map(|at| (at.line, at.column));
    /// let key = input.place_key(&glob).map(|at| (at.line, at.column));
    /// assert_eq!(value, Some((3, 5)));
    /// assert_eq!(key, Some((2, 3)));
    /// # }
    /// ```
    #[must_use]
    pub fn place_key(&self, path: &FieldPath) -> Option<Place> {
        self.places(slice::from_ref(path), At::Key).pop().flatten()
    }

    /// A [`Problem`] for `path` with `message`, placed as [`Input::place`]
    /// places it, with the path printed as garde prints a field.
    ///
    /// A CLI uses it for a check that runs after the read, such as one that
    /// needs a file the config names.
    #[must_use]
    pub fn problem_at(&self, path: &FieldPath, message: impl Into<String>) -> Problem {
        let place = self.place(path);
        Problem {
            line: place.map(|at| at.line),
            column: place.map(|at| at.column),
            field: path.to_string(),
            message: message.into(),
        }
    }

    /// The place of each path, with the frontmatter lines added.
    fn places(&self, paths: &[FieldPath], at: At) -> Vec<Option<Place>> {
        let limits = limits::effective(&self.limits);
        if self.text.len() > limits.bounds().input_bytes {
            return vec![None; paths.len()];
        }
        let read = self
            .kept
            .tree(self.format, &self.text, &limits, Reading::Listing);
        let Ok(tree) = read.as_ref() else {
            return vec![None; paths.len()];
        };
        paths
            .iter()
            .map(|path| {
                let (line, column) =
                    tree::position(&self.text, tree.nearest(path.steps(), at).start);
                Some(Place::new(line + self.lines_before, column))
            })
            .collect()
    }
}

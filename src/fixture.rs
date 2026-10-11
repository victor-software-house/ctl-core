//! Files for integration tests, written through one call instead of
//! `fs::create_dir_all`, `fs::write`, and an `expect` per file.
//!
//! A [`Suite`] owns a temporary directory named after the test file, and each
//! [`Suite::scenario`] owns its own directory inside it. Both are removed when
//! the value drops, so a test cleans up by going out of scope. A write that
//! fails panics with the path it could not write: a broken fixture is a broken
//! test, and the message says which file.

use std::fs;
use std::marker::PhantomData;
use std::path::{Component, Path, PathBuf};

use tempfile::{Builder, TempDir};

/// A temporary directory for one test file.
pub struct Suite {
    name: String,
    root: Root<'static>,
}

impl Suite {
    /// A fresh directory whose name starts with `name`.
    ///
    /// # Panics
    ///
    /// When the system temporary directory cannot hold a new directory.
    #[must_use]
    #[track_caller]
    pub fn new(name: &str) -> Self {
        let dir = Builder::new()
            .prefix(&format!("{name}-"))
            .tempdir()
            .unwrap_or_else(|error| panic!("create a temporary directory for {name}: {error}"));
        Self {
            name: name.to_owned(),
            root: Root {
                dir,
                suite: PhantomData,
            },
        }
    }

    /// A fresh directory for one scenario, inside this suite's, removed when
    /// the returned [`Root`] drops. The scenario borrows the suite, because
    /// dropping the suite first would delete a directory still in use:
    ///
    /// ```compile_fail,E0505
    /// let suite = ctl_core::fixture::Suite::new("early");
    /// let root = suite.scenario("orphan");
    /// drop(suite);
    /// root.file("a.md", "a");
    /// ```
    ///
    /// # Panics
    ///
    /// When the suite's directory cannot hold a new directory.
    #[must_use]
    #[track_caller]
    pub fn scenario(&self, name: &str) -> Root<'_> {
        let dir = Builder::new()
            .prefix(&format!("{name}-"))
            .tempdir_in(self.root.path())
            .unwrap_or_else(|error| panic!("create {} scenario {name}: {error}", self.name));
        Root {
            dir,
            suite: PhantomData,
        }
    }

    /// The suite's own directory.
    #[must_use]
    pub fn root(&self) -> &Root<'static> {
        &self.root
    }
}

/// A temporary directory a test writes files into, alive no longer than the
/// suite it sits in.
pub struct Root<'suite> {
    dir: TempDir,
    suite: PhantomData<&'suite ()>,
}

impl Root<'_> {
    /// Where the directory is.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Write `body` to `relative`, creating its parent directories, and return
    /// the full path.
    ///
    /// # Panics
    ///
    /// When `relative` is absolute or climbs out with `..`, or the file cannot
    /// be written.
    #[track_caller]
    pub fn file(&self, relative: impl AsRef<Path>, body: impl AsRef<[u8]>) -> PathBuf {
        let path = self.inside(relative.as_ref());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("create {}: {error}", parent.display()));
        }
        fs::write(&path, body).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
        path
    }

    /// Read a file back as text.
    ///
    /// # Panics
    ///
    /// When `relative` is absolute or climbs out with `..`, or the file cannot
    /// be read as UTF-8.
    #[must_use]
    #[track_caller]
    pub fn read(&self, relative: impl AsRef<Path>) -> String {
        let path = self.inside(relative.as_ref());
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    }

    /// `relative` joined onto this directory. Joining an absolute path would
    /// replace the root, so a fixture could write anywhere.
    #[track_caller]
    fn inside(&self, relative: &Path) -> PathBuf {
        assert!(
            relative
                .components()
                .all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
            "{} must stay inside the fixture root",
            relative.display()
        );
        self.path().join(relative)
    }
}

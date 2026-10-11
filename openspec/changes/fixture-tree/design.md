# Design: fixture tree

## Decisions

### 1. A cargo feature, not `cfg(test)`

`cfg(test)` code is invisible to other crates, so verctl could not use it. A
separate crate was considered. It lost because the row asks for no new
runtime crate, and a feature keeps `tempfile` out of every build that does
not enable it.

### 2. Panics, not `Result`

A fixture that cannot be written is a broken test. Returning `Result` would
put the `expect` back at every call site, which is the repetition this
removes. Each panic names the path, and `#[track_caller]` points at the test
line.

### 3. Scenario directories inside the suite's

A scenario directory sits inside its suite's, so one test file's files share
one parent that a reader can find. Sibling directories in the system temp
directory were considered. They lost because nothing would group a run's
leftovers if a process is killed before the drop.

### 4. Paths stay inside the root

Joining an absolute path onto the root replaces it, so a typo could write
anywhere. `file` and `read` accept only normal components and `.`.

## Override and disable

Nothing is automatic. A consumer opts in with the `fixture` feature on its
ctl-core dev-dependency.

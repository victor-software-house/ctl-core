# fixture-tree Specification

## Purpose
Temporary file trees for integration tests: one suite directory per test file, a scenario per case that cannot outlive its suite, and one call per file.

## Requirements

### Requirement: A fixture file is written in one call

`Root::file(path, body)` SHALL create the parent directories of `path` under
the root, write `body`, and return the full path. `Root::read(path)` SHALL
return the file's text.

#### Scenario: A nested config file

- **WHEN** a test calls `root.file(".ctl/ver.yaml", body)` on a fresh scenario
- **THEN** `.ctl/` exists under the scenario, the returned path is
  `<scenario>/.ctl/ver.yaml`, and `root.read(".ctl/ver.yaml")` equals `body`

### Requirement: Suites and scenarios own their temporary directories

`Suite::new(name)` SHALL create a temporary directory whose name starts with
`name`. `suite.scenario(name)` SHALL create a separate directory inside the
suite's. Each directory SHALL be removed when its owner drops.

#### Scenario: Two scenarios do not share files

- **WHEN** a test writes `Cargo.toml` in `suite.scenario("first")`
- **THEN** `suite.scenario("second")` has no `Cargo.toml`, and both
  directories sit inside the suite's directory

#### Scenario: A dropped scenario is gone

- **WHEN** a scenario holding `a.md` drops
- **THEN** `a.md` no longer exists and the suite's directory still does

#### Scenario: A scenario cannot outlive its suite

- **WHEN** a test drops a `Suite` and then writes through one of its scenarios
- **THEN** the test does not compile (E0505), because the scenario borrows
  the suite

### Requirement: A fixture never writes outside its root

`Root::file` and `Root::read` SHALL panic, writing nothing, when the path is
absolute or contains `..`.

#### Scenario: A climbing path

- **WHEN** a test calls `root.file("../outside.md", "x")`
- **THEN** the call panics and no `outside.md` is written next to the root

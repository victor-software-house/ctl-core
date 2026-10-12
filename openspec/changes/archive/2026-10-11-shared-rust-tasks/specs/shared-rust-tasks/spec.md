## ADDED Requirements

### Requirement: ctl-core ships the standard Rust tasks as one TOML task file

`tasks/rust.toml` SHALL define `check:toolchain`, `format`, `lint`, `test`,
`test:doc`, `deny`, `deny:advisories`, `deps`, `semver`, and `verify`, and
ctl-core's `mise.toml` SHALL include it by path.

#### Scenario: ctl-core's own tasks

- **WHEN** `mise tasks info verify --json` runs in ctl-core
- **THEN** the task's source is `tasks/rust.toml` and its dir is the
  repository root

### Requirement: verify depends on the repository's own check:features

`verify` SHALL depend on `check:features` by name, so it runs the task the
including repository defines.

#### Scenario: A consumer with its feature sets

- **WHEN** a repository includes `tasks/rust.toml` by `git::` URL at a
  release tag and defines `check:features`
- **THEN** `mise tasks deps verify` lists `check:features`, and
  `mise run verify` runs the repository's feature checks

#### Scenario: Experimental off

- **WHEN** a repository includes the file with mise's `experimental` setting
  off
- **THEN** `mise run verify` fails with a missing task, not a pass

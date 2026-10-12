## ADDED Requirements

### Requirement: verify depends on the repository's own check:repo

`verify` SHALL depend on `check:repo` by name, so it runs the checks the
including repository adds to the shared ones.

#### Scenario: A consumer with checks of its own

- **WHEN** a repository includes `tasks/rust.toml` at `v0.6.14` and defines
  `check:repo` with `depends = ["specs", "lint:typos"]`
- **THEN** `mise tasks deps verify` lists `check:repo`, and
  `mise run verify` runs `specs` and `lint:typos`

#### Scenario: A repository without check:repo

- **WHEN** a repository includes `tasks/rust.toml` at `v0.6.14` and does not
  define `check:repo`
- **THEN** `mise run verify` fails with a missing task, not a pass

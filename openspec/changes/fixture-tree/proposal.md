# Proposal: fixture tree

Queue row: CTC-006.

## Why

Every ctl CLI writes its test fixtures by hand. verctl's integration tests
call `fs::write` 79 times across nine files, each with its own
`create_dir_all` and `expect`, and build their temporary directories with
`TempDir::new()` in every test. qctl and forkctl repeat the same pattern
against their own `tempfile` pins. The row asks for one helper here instead
of a wrapper in each CLI.

## What Changes

1. A new `fixture` feature exposes `ctl_core::fixture`.
2. `Suite::new(name)` owns a temporary directory named after the test file.
   `suite.scenario(name)` owns its own directory inside it. Both are removed
   when they drop.
3. `Root::file(path, body)` writes a file, creating its parent directories,
   and returns the full path. `Root::read(path)` reads it back.
4. A path that is absolute or climbs out with `..` panics instead of writing
   outside the fixture.
5. `tempfile` 3.27.0 becomes an optional dependency, only under `fixture`.

## Capabilities

### New Capabilities

- `fixture-tree`: suites, scenarios, and fixture writes for integration tests.

### Modified Capabilities

None.

## Impact

1. A patch release of ctl-core, as for `input`: a new feature, nothing
   existing changes.
2. Consumers enable `fixture` on their ctl-core dev-dependency. verctl first,
   then qctl and forkctl, drop their copied `fs::write` fixtures in their own
   repositories after pinning that version.

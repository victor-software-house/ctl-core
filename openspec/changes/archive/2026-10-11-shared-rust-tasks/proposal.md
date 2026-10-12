# Proposal: shared Rust tasks

## Why

Each ctl repository writes the same ten Rust tasks in its own
`mise.dev.toml`. A fix then has to be copied into every repository by hand.
[#94] fixed `check:toolchain`, which passed when `cargo metadata` failed, in
ctl-core alone. The other repositories still carry the broken text. mise can
load tasks from a TOML file in another repository at a pinned ref, the way
the `q` task already reaches these repositories (CTC-016).

## What Changes

1. ctl-core ships `tasks/rust.toml`: `check:toolchain`, `format`, `lint`,
   `test`, `test:doc`, `deny`, `deny:advisories`, `deps`, `semver`, and
   `verify`.
2. ctl-core's `mise.toml` includes that file by path, and its
   `mise.dev.toml` keeps only `check:features`, `commit-msg`, and `commits`.
3. Another repository adds the file to `task_config.includes` as a `git::`
   URL at a ctl-core release tag, turns on mise's `experimental` setting,
   and deletes its own copies.
4. One consumer repository moves onto the include as the proof.

## Capabilities

### New Capabilities

- `shared-rust-tasks`: the task file, how a repository includes it, and how
  `verify` reaches the repository's own `check:features`.

### Modified Capabilities

None.

## Impact

1. New file `tasks/rust.toml`. It is not compiled into the crate.
2. ctl-core's `mise.toml` and `mise.dev.toml` change; the tasks themselves
   do not.
3. Consumers need mise's `experimental` setting for `git::` includes.

[#94]: https://github.com/victor-software-house/ctl-core/pull/94

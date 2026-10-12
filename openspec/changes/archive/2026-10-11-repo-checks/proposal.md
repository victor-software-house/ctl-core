# Proposal: repository checks in the shared verify

## Why

The shared `verify` in `tasks/rust.toml` has a fixed list of dependencies.
A repository with checks of its own has to override `verify` and repeat the
whole list. [yamled#19], the first consumer, did that to add five checks
(TOML format, Markdown lint, spelling, workflow lint, OpenSpec validation).
Each override is a copy of the list that drifts when the shared list changes
(CTC-017).

## What Changes

1. The shared `verify` also depends on `check:repo`, a task each including
   repository defines, the same way it already depends on `check:features`.
2. ctl-core defines its own `check:repo`, which does nothing: ctl-core has
   no checks beyond the shared ones.
3. yamled replaces its `verify` override with a `check:repo` that depends on
   its five checks, after ctl-core 0.6.14 is released.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `shared-rust-tasks`: `verify` depends on `check:repo` as well as
  `check:features`.

## Impact

1. Every repository that includes `tasks/rust.toml` at 0.6.14 or later must
   define `check:repo`, or `verify` fails with a missing task.
2. No crate code changes.

[yamled#19]: https://github.com/victor-software-house/yamled/pull/19

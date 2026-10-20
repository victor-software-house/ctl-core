# Proposal

## Why

ctl-core shipped the shared Rust task catalog, `tasks/rust.toml`, and the template drift check, `.github/actions/copier-check/`, before the public [actions] repository existed.
Both now live there.
On 2026-10-20 no default branch in the organization includes either copy from ctl-core: a scan of every non-archived repository's mise configuration and workflow files found no reference, and a code search found none outside ctl-core.
ctl-core still carries both copies and `check:rust-tasks`, a check that tests a file nothing includes.
This change is queue row CTC-026.

## What Changes

1. Delete `tasks/rust.toml`, `.github/actions/copier-check/`, and the `check:rust-tasks` file task.
2. Drop the `.taplo.toml` exclusion for `tasks/rust.toml` and the `check:local` dependency on `check:rust-tasks`.
3. Point `AGENTS.md` at the actions repository for the shared tasks and the drift check.
4. An include pinned to an existing ctl-core tag keeps working, because tags never move.
   No later tag carries the file.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `shared-rust-tasks`: every requirement is removed and the capability is retired, because ctl-core no longer ships the file. The [actions] repository owns the catalog now.

## Impact

1. Files: `tasks/rust.toml`, `.github/actions/copier-check/action.yml`, `.github/actions/copier-check/check.sh`, `mise-tasks/check/rust-tasks`, `mise-tasks/check/local.sh`, `.taplo.toml`, `AGENTS.md`, `openspec/specs/shared-rust-tasks/`.
2. The published crate, its API, and its tests do not change.
3. `verify` keeps the same tasks: `mise.toml` already includes them from the [actions] repository.

[actions]: https://github.com/victor-software-house/actions/tree/1b6e52f1c2783dec2481fe281850aeb0161e92bd

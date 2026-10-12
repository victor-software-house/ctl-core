# Proposal: served Rust baseline

## Why

Six ctl repositories ([ctl-core], [qctl], [forkctl], [verctl], [yamled],
[unixd]) each keep their own copy of the same Rust configuration. The copies
have already drifted. On 2026-10-12, qctl had no `rust-toolchain.toml` or
`rustfmt.toml`, and forkctl had no `mise.dev.toml`. Each policy change, such
as the move to pinned stable 1.99.0 in [#90], must then be repeated by hand in
every repository. ctl-core is the reference implementation, so it should
serve these files and let a repository check that its copies still match
(CTC-015).

## What Changes

1. A new feature `baseline` embeds templates for five whole files:
   `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, and
   `lefthook.yml`.
2. It also embeds two marked blocks inside files the repository otherwise
   owns. The first is the lints table in `Cargo.toml` (`[lints.*]`, or
   `[workspace.lints.*]` in a workspace). The second is the `[tools]`
   toolchain entries in `mise.dev.toml`.
3. A repository opts in with `.ctl/rust.yaml`. ctl-core owns its schema and
   reads it through `input`. It states the layout, whether the crate is
   published, whether it uses tokio, append-only entries such as
   extra cargo-deny bans, and the files the repository keeps for itself.
4. `baseline::render(root, &config)` returns one `served::Served` per file,
   ready to write or to compare. A file with a marked block is returned whole,
   with only the block replaced.
5. ctl-core renders its own `.ctl/rust.yaml` in a test and fails `verify`
   when any of its files differs from the render.
6. ctl-core stays a library. verctl writes the files and reports drift, as
   part of its one served-file mechanism. `.github/workflows/ci.yml` stays
   verctl's.

## Capabilities

### New Capabilities

- `served-baseline`: the baseline templates, the repository schema, the
  marked blocks, and the self-check.

### Modified Capabilities

None. `served::render` and `Served::write` keep their contract.

## Impact

1. New code in `src/baseline.rs` and templates under `templates/baseline/`.
2. A new file, `.ctl/rust.yaml`, in ctl-core and later in each consumer.
3. Marker comments added to ctl-core's `Cargo.toml` and `mise.dev.toml`.
4. No new dependency. The feature builds on `served` and `input`.

[ctl-core]: https://github.com/victor-software-house/ctl-core
[qctl]: https://github.com/victor-software-house/qctl
[forkctl]: https://github.com/victor-software-house/forkctl
[verctl]: https://github.com/victor-software-house/verctl
[yamled]: https://github.com/victor-software-house/yamled
[unixd]: https://github.com/victor-software-house/unixd
[#90]: https://github.com/victor-software-house/ctl-core/pull/90

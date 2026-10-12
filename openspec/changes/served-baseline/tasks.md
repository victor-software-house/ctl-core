# Tasks

- [x] 1. The `.ctl/rust.yaml` schema in `src/baseline.rs`, loaded through
  `input`. Proof: tests for a valid file, an unknown field, and an entry
  without a reason, under `mise run verify` on a build runner.
- [x] 2. Templates for the five whole files and the two blocks, taken from
  ctl-core's own files at [02941ff]. Proof: the self-check in task 4.
- [x] 3. `baseline::render` with block splicing and owned files. Proof: tests
  for repository content surviving, a workspace layout, missing markers, and
  an owned file.
- [x] 4. ctl-core's own `.ctl/rust.yaml`, markers in `Cargo.toml` and
  `mise.dev.toml`, and `tests/baseline.rs`. Proof: the test passes, and fails
  when one template line changes alone.
- [x] 5. `check:features` covers `--features baseline`. Proof: `mise run
  check:features` on a build runner.
- [x] 6. AGENTS.md section and a patch changeset. The README lists no
  features, so it does not change. Proof: the files in the diff.

## Evidence

2026-10-12 UTC. `mise run verify` passed on a build runner on [ab7b23c] and
on the commit that ticks these tasks. `tests/baseline.rs` covers: ctl-core's
seven files equal their render; a published crate gets all seven files,
the pinned stable, and cargo-semver-checks, and an unpublished one does not
get the tool; tokio adds the lock guards and the channel ban; an appended
license and ban reach `deny.toml` with their reasons; an unknown field, an
entry without a reason, and a quote in a reason are refused; a block is
replaced and the rest of `Cargo.toml` kept byte for byte; a workspace gets
`[workspace.lints.*]`; a file without its markers names the file and the
marker; an owned file is skipped and reported. With one line appended to
`templates/baseline/clippy.toml.jinja` alone, the self-check fails naming
`clippy.toml`. `check:features` builds `--features baseline` alone.

Not done here: verctl writing and checking these files for other
repositories, which is VER-045, after verctl 0.4.0.

[02941ff]: https://github.com/victor-software-house/ctl-core/commit/02941ff7bd0d1d7adf9fb07db59599ecf3da66f0
[ab7b23c]: https://github.com/victor-software-house/ctl-core/commit/ab7b23c3d6be5d7e87988769dedd4eacbef75ce9

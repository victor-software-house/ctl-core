# Tasks

- [ ] 1. The `.ctl/rust.yaml` schema in `src/baseline.rs`, loaded through
  `input`. Proof: tests for a valid file, an unknown field, and an entry
  without a reason, under `mise run verify` on a build runner.
- [ ] 2. Templates for the five whole files and the two blocks, taken from
  ctl-core's own files at [02941ff]. Proof: the self-check in task 4.
- [ ] 3. `baseline::render` with block splicing and owned files. Proof: tests
  for repository content surviving, a workspace layout, missing markers, and
  an owned file.
- [ ] 4. ctl-core's own `.ctl/rust.yaml`, markers in `Cargo.toml` and
  `mise.dev.toml`, and `tests/baseline.rs`. Proof: the test passes, and fails
  when one template line changes alone.
- [ ] 5. `check:features` covers `--features baseline`. Proof: `mise run
  check:features` on a build runner.
- [ ] 6. AGENTS.md section, README feature line, and a patch changeset.
  Proof: the files in the diff.

## Evidence

[02941ff]: https://github.com/victor-software-house/ctl-core/commit/02941ff7bd0d1d7adf9fb07db59599ecf3da66f0

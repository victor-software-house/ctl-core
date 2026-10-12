# Tasks

- [ ] 1. `verify` in `tasks/rust.toml` depends on `check:repo`, and
  ctl-core's `mise.dev.toml` defines it. Proof: `mise tasks deps verify`
  lists `check:repo`, and `mise run verify` passes on a build runner.
- [ ] 2. A repository without `check:repo` fails. Proof: `mise run verify`
  in a directory that includes the file and defines only `check:features`
  exits non-zero with a missing-task error.
- [ ] 3. Release ctl-core 0.6.14. Proof: the tag exists and its
  `tasks/rust.toml` matches main.
- [ ] 4. yamled replaces its `verify` override with `check:repo` at
  `v0.6.14`. Proof: its `mise run verify` passes on a build runner and in
  CI.
- [ ] 5. AGENTS.md, the task file's header, and a patch changeset. Proof:
  the files in the diff.

# Tasks

- [ ] 1. `tasks/rust.toml` with the ten tasks, included by ctl-core's
  `mise.toml`; `mise.dev.toml` keeps `check:features`, `commit-msg`, and
  `commits`. Proof: `mise tasks info verify --json` names the file, and
  `mise run verify` passes on a build runner.
- [ ] 2. The `git::` form resolves from a pushed ref. Proof: a scratch
  directory that includes the file by URL lists all ten tasks, and
  `mise tasks deps verify` lists its own `check:features`.
- [ ] 3. Release ctl-core with the file at a tag. Proof: the tag exists and
  its `tasks/rust.toml` matches main.
- [ ] 4. One consumer moves onto the include at that tag. Proof: its
  `mise run verify` passes on a build runner and in CI.
- [ ] 5. AGENTS.md and a patch changeset. Proof: the files in the diff.

## Evidence

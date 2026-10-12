# Tasks

- [x] 1. `tasks/rust.toml` with the ten tasks, included by ctl-core's
  `mise.toml`; `mise.dev.toml` keeps `check:features`, `commit-msg`, and
  `commits`. Proof: `mise tasks info verify --json` names the file, and
  `mise run verify` passes on a build runner.
- [x] 2. The `git::` form resolves from a pushed ref. Proof: a scratch
  directory that includes the file by URL lists all ten tasks, and
  `mise tasks deps verify` lists its own `check:features`.
- [ ] 3. Release ctl-core with the file at a tag. Proof: the tag exists and
  its `tasks/rust.toml` matches main.
- [ ] 4. One consumer moves onto the include at that tag. Proof: its
  `mise run verify` passes on a build runner and in CI.
- [x] 5. AGENTS.md and a patch changeset. Proof: the files in the diff.

## Evidence

2026-10-12 UTC, tasks 1, 2, and 5. On a build runner, `mise tasks info
verify --json` in ctl-core gave source `tasks/rust.toml` and the repository
root as dir, and `mise run verify` passed on [8019275], running all eight
dependencies, `check:features` included. From an empty directory that
includes the file by `git::` URL and defines its own `check:features`, with
mise 2026.10.7 and 2026.10.3: all ten tasks load, `mise tasks deps verify`
lists `check:features`, a dry run of `verify` runs the directory's own
`check:features`, and the task dir is that directory. A full SHA and a
branch name both resolve; the short SHA `8019275` fails. With
`MISE_EXPERIMENTAL=0`, mise warns that it skipped the include, and
`mise run verify` exits 1 with `no task verify found`.

Not yet done: tasks 3 and 4, the release and the consumer proof.

[8019275]: https://github.com/victor-software-house/ctl-core/commit/80192754019a4a2efb1bd8d07f91f694ca16c3c8

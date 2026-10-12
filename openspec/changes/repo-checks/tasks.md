# Tasks

- [x] 1. `verify` in `tasks/rust.toml` depends on `check:repo`, and
  ctl-core's `mise.dev.toml` defines it. Proof: `mise tasks deps verify`
  lists `check:repo`, and `mise run verify` passes on a build runner.
- [x] 2. A repository without `check:repo` fails. Proof: `mise run verify`
  in a directory that includes the file and defines only `check:features`
  exits non-zero with a missing-task error.
- [ ] 3. Release ctl-core 0.6.14. Proof: the tag exists and its
  `tasks/rust.toml` matches main.
- [ ] 4. yamled replaces its `verify` override with `check:repo` at
  `v0.6.14`. Proof: its `mise run verify` passes on a build runner and in
  CI.
- [x] 5. AGENTS.md, the task file's header, and a patch changeset. Proof:
  the files in the diff.

## Evidence

2026-10-12 UTC, tasks 1, 2, and 5. On a fresh Linux x86_64 build runner
with mise 2026.10.3, `mise tasks deps verify` in ctl-core listed
`check:repo` and `check:features`, and the pre-push `mise run verify` passed
on [f216676], running ctl-core's `check:repo`. A directory that includes
`tasks/rust.toml` at that commit and defines only `check:features` failed
`mise run verify` with exit 1 and `task not found: check:repo`.

Not yet done: tasks 3 and 4, the release and yamled's move.

[f216676]: https://github.com/victor-software-house/ctl-core/commit/f216676f5701d73aafd3098fb60c51288a7d67cd

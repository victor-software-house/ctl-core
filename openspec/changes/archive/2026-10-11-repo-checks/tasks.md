# Tasks

- [x] 1. `verify` in `tasks/rust.toml` depends on `check:repo`, and
  ctl-core's `mise.dev.toml` defines it. Proof: `mise tasks deps verify`
  lists `check:repo`, and `mise run verify` passes on a build runner.
- [x] 2. A repository without `check:repo` fails. Proof: `mise run verify`
  in a directory that includes the file and defines only `check:features`
  exits non-zero with a missing-task error.
- [x] 3. Release ctl-core 0.6.14. Proof: the tag exists and its
  `tasks/rust.toml` matches main.
- [x] 4. yamled replaces its `verify` override with `check:repo` at
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

2026-10-12 UTC, task 3. ctl-core [0.6.14] published to crates.io at
02:05:18 UTC; the tag [v0.6.14] holds the same `tasks/rust.toml` as main,
with `check:repo` in `verify`.

2026-10-12 UTC, task 4. [yamled#21] includes the file at `v0.6.14`, drops
its `verify` override, and defines `check:repo` over its five checks. On a
fresh Linux x86_64 instance with no system node, `mise tasks deps verify`
listed `check:repo`, and the pre-push `mise run verify` passed; CI passed
on the merged head.

[f216676]: https://github.com/victor-software-house/ctl-core/commit/f216676f5701d73aafd3098fb60c51288a7d67cd
[0.6.14]: https://crates.io/crates/ctl-core/0.6.14
[v0.6.14]: https://github.com/victor-software-house/ctl-core/tree/v0.6.14
[yamled#21]: https://github.com/victor-software-house/yamled/pull/21

# Tasks

- [x] 1. Plan: this change, and the CTC-022 row added with `qctl add`. Proof:
  `openspec validate env-bound-flags --strict` and `mise run q check` pass.
- [x] 2. Add `EnvPrefix` in `src/flags/env.rs`, and export it
  from `flags`, the crate root, and the prelude. Proof: the unit tests in
  `src/flags/env.rs` pass on a build runner.
- [x] 3. Print `[env: NAME]` in ctl-core help. Proof:
  `help_names_each_variable` in `tests/env_flags.rs` passes.
- [x] 4. Add `env_under_flags` in `src/flags/env.rs`, export it, and call it
  in `App` before the parse. Proof: the negation, conflict, group,
  exclusive, false-variable, and two-variable scenarios in
  `tests/env_flags.rs` pass.
- [x] 5. Prove the parse through the environment in a child process. Proof:
  every test in `tests/env_flags.rs` passes, and `mise run verify` passes on
  a build runner.
- [x] 6. Document the binding and the precedence rule in AGENTS.md, and add
  the changeset. Proof: the pre-push `verify` passes on the pushed commit.
- [x] 7. Open the PR, wait for CI and Kody, and answer each thread with the
  commit that fixes it. Proof: CI green on the last push, no open thread.
  - 2026-10-24 UTC: [PR 130][ctl-core#130] merged as [d794772] on
    2026-10-15 with every check green on its last push, [5987117], and no
    open review thread.

## Evidence

2026-10-15 UTC, tasks 2 to 6. On a Linux amd64 Namespace build runner,
`mise run verify` exited 0 on the tree of the implementation: format, clippy,
`check:features`, unit, integration, and doc-tests with 0 failures,
cargo-deny licenses, bans, and sources, cargo-machete, and
`openspec validate --all --strict`. That covers the 4 unit tests in
`src/flags/env.rs` and the 14 tests in `tests/env_flags.rs`, each scenario in
a child process with its variables set, plus `App::run_from` with
`ENVTOY_PR=true` and `ENVTOY_AFTER=a1` beside `--no-pr --before b1`.

Before the redesign, a run with only `switch(on, off) -> on && !off` failed
both negation tests with `ArgumentConflict` ("`--no-pr` cannot be used with
`--pr`"), which showed that clap refuses the variable before `switch` runs.

Not verified: the change against a consuming CLI. verctl, qctl, and forkctl
adopt the attribute after the release that ships this.

[ctl-core#130]: https://github.com/victor-software-house/ctl-core/pull/130
[d794772]: https://github.com/victor-software-house/ctl-core/commit/d794772
[5987117]: https://github.com/victor-software-house/ctl-core/commit/5987117

# Tasks

- [x] 1. Plan: this change, and the CTC-031 row added with `qctl add`.
  Proof: `openspec validate script-snapshots --strict` and
  `mise run q check` pass.
  - 2026-11-08 UTC: both passed, with `mise run check:repo`.
- [x] 2. Pin `insta = "=1.49.0"` with no default features as a
  dev-dependency, `cargo-insta` 1.49.0 in `mise.dev.toml`, and
  `INSTA_UPDATE=no` in `mise.ci.toml`.
  Proof: `Cargo.lock` adds only `insta` and `similar`.
  - 2026-11-08 UTC: `cargo metadata` locked `insta` 1.49.0 and `similar`
    2.7.0, and nothing else.
- [x] 3. Pin zsh, fish, and shellcheck in the `test` profile,
  `mise.test.toml`, and lock them for linux-x64 and macos-arm64.
  Proof: `mise lock --dry-run --json` prints `[]`, and `mise run test`
  installs the tools cold in CI.
  - 2026-11-08 UTC: locked for both platforms, first as `[tools]`, then
    moved to `[tasks.test]` on review; the dry run printed `[]` after the
    move. On macos-arm64 the locked versions installed and ran.
  - 2026-11-08 UTC: a cold CI run failed on the task tools
    ([run 42433739166][run-cold]): mise reads a task tool's conda
    dependencies from no lockfile (design decision 4). The tools moved to
    lazy `[tools]` entries; the `MISE_ENV=dev` dry run prints `[]`.
  - 2026-11-08 UTC: CI then failed to start zsh and fish from the tests
    ([run 42434896632][run-wrapped]): mise takes its shims off the `PATH`
    of cargo, a command wrapper. `test` now depends on
    `install:test-tools`. On a cold Linux x64 sandbox with `MISE_ENV=ci`,
    a task after that dependency found all three installed tools with the
    shims off `PATH`.
  - 2026-11-08 UTC: review rejected the lazy tools and the install task as
    a workaround. The tools moved to the `test` profile: the shared
    workflow gained a `test-profiles` input ([actions#55][actions-55]), and
    a template update set `test_profiles: [test]`, which adds `test` to
    `.miserc.toml` and passes it to CI's Build and test job. The locks were
    generated with the default profiles; `mise lock --dry-run --json` prints
    `[]` with the default profiles and with `MISE_ENV` set to `dev`, `test`,
    and `release`.
  - 2026-11-08 UTC: [#187] landed the `test` profile. Its CI
    [run 42447332218][run-187] on [5f320ff] found no mise cache for the
    `ci-test` lock, installed zsh 5.9.2, fish 4.9.3, and shellcheck 0.11.0
    from the lock, and passed `mise run verify`.
- [x] 4. Add `tests/support/shells.rs`, and move the completion script and
  help tests to snapshots that pass their shell's check.
  Proof: the completion tests pass with `INSTA_UPDATE=no`, locally and in
  CI.
  - 2026-11-08 UTC: on a macOS arm64 workstation, all 11 completion tests
    passed with `INSTA_UPDATE=no`, and clippy on the test crate passed
    with `-D warnings`. The bash, zsh, and fish snapshots were written by
    hand from `clap_complete`'s templates and matched on the first run; the
    help snapshot was rendered by insta. CI has not run yet.
  - 2026-11-08 UTC: CI [run 42447332050][run-181] passed on the last push
    to [#181], [e9baad7], with `INSTA_UPDATE=no` from `mise.ci.toml`.
- [x] 5. State the closed `Shell` set in its rustdoc, and record the
  testing rule in `AGENTS.md`.
  Proof: `mise run verify` passes in CI on the pushed commit.
  - 2026-11-08 UTC: `mise run verify` passed in CI
    [run 42447332050][run-181] on [e9baad7]. The rustdoc is on `Shell` in
    `ctl-core/src/shell.rs`, and the rule is the Snapshots section of
    `AGENTS.md`.
- [x] 6. Open the PR ready, wait for CI and Kody, and answer each thread
  in the top PR before the stack merges.
  Proof: CI green on the last push.
  - 2026-11-08 UTC: [#181] passed CI on its last push, as task 4 records.
    [#184]'s last push, [d411327], failed Build and test in
    [run 42447332884][run-184]: mise found no linux-x64 lock entry for a
    conda dependency of zsh and fish. [#187] replaced that approach with
    the `test` profile, and its CI [run 42447332218][run-187] passed on its
    last push, [5f320ff]. Kody's thread on [#184] resolved once [#187]
    removed the lazy tools, and its thread on [#187] was answered with
    evidence and withdrawn. The stack merged through the merge queue, with
    CI [run 42448670375][run-queue] green on [b670378].

[actions-55]: https://github.com/victor-software-house/actions/pull/55
[run-cold]: https://github.com/victor-software-house/ctl-core/actions/runs/42433739166
[run-wrapped]: https://github.com/victor-software-house/ctl-core/actions/runs/42434896632
[#181]: https://github.com/victor-software-house/ctl-core/pull/181
[#184]: https://github.com/victor-software-house/ctl-core/pull/184
[#187]: https://github.com/victor-software-house/ctl-core/pull/187
[e9baad7]: https://github.com/victor-software-house/ctl-core/commit/e9baad7c8319c1321270c53363bdf633c069d31a
[d411327]: https://github.com/victor-software-house/ctl-core/commit/d4113275f7b47ecda43ada86c31ef907d59886b6
[5f320ff]: https://github.com/victor-software-house/ctl-core/commit/5f320ff41fe10f0e3227f55a458b389c493afe07
[b670378]: https://github.com/victor-software-house/ctl-core/commit/b6703788d2ebb1a04113bc1cc00f4901cb320951
[run-181]: https://github.com/victor-software-house/ctl-core/actions/runs/42447332050
[run-184]: https://github.com/victor-software-house/ctl-core/actions/runs/42447332884
[run-187]: https://github.com/victor-software-house/ctl-core/actions/runs/42447332218
[run-queue]: https://github.com/victor-software-house/ctl-core/actions/runs/42448670375

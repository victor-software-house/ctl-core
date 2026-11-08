# Tasks

- [x] 1. Research how current CLIs ship completion, and commit it as
  [`docs/research/shell-completion.md`][research]. Proof: the markdown lint
  passes on the file.
  - 2026-11-08 UTC: committed as [7897e70]; `rumdl check` reported no
    issues.
- [x] 2. Plan: this change, and the CTC-030 row added with `qctl add`.
  Proof: `openspec validate completion-verb --strict` and
  `mise run q check` pass.
  - 2026-11-08 UTC: both passed on [18b76f9].
- [x] 3. Add `Shell` in `src/shell.rs` behind the `shell` feature. Proof:
  the parse scenarios in `tests/completion.rs` pass in CI.
  - 2026-11-08 UTC: CI [run 42431161985][run] passed on [7cc88ad], including
    the `completion` tests and every `check:features` set.
- [x] 4. Add `CompletionCommand`, `Shell::script`, `Script`, and `view` in
  `src/completion.rs` behind the `completion` feature, with
  `clap_complete` pinned to `=4.6.11`. Proof: the script, help, and view
  tests in `tests/completion.rs` pass in CI.
  - 2026-11-08 UTC: passed in CI [run 42431161985][run] on [7cc88ad].
    [script-snapshots] later moved the script and help tests to snapshots.
- [x] 5. Add `App::completion()`. Proof: the environment tests in
  `tests/completion.rs`, each in a child process with `COMPLETE` set, pass
  in CI.
  - 2026-11-08 UTC: passed in CI [run 42431161985][run] on [7cc88ad].
- [x] 6. Add `shell` and `completion` to `check:features`, document both
  features in the crate docs and `AGENTS.md`, and add the changeset.
  Proof: `mise run verify` passes in CI on the pushed commit.
  - 2026-11-08 UTC: CI [run 42431161985][run] passed on [7cc88ad], including
    the `completion` tests and every `check:features` set.
- [ ] 7. Mark the PR ready, wait for CI and Kody, and answer each thread.
  Proof: CI green on the last push.

[7897e70]: https://github.com/victor-software-house/ctl-core/commit/7897e70c03967ffc3c96cfaf8bb1fb04f3576c55
[research]: ../../../docs/research/shell-completion.md
[18b76f9]: https://github.com/victor-software-house/ctl-core/commit/18b76f94173226129cb2402a137eab32accfbbdb
[7cc88ad]: https://github.com/victor-software-house/ctl-core/commit/7cc88ad533f286fbedd157d67d7254c948a52fbf
[run]: https://github.com/victor-software-house/ctl-core/actions/runs/42431161985
[script-snapshots]: ../script-snapshots/proposal.md

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
- [x] 7. Mark the PR ready, wait for CI and Kody, and answer each thread.
  Proof: CI green on the last push.
  - 2026-11-08 UTC: CI [run 42447333133][run-180] passed on the last push
    to [#180], [a1a0fcd]. Kody's one thread was fixed on the stack top in
    [#188], whose CI [run 42447452161][run-188] passed on [e02e9e6], and
    the thread is resolved. The stack merged through the merge queue, with
    CI [run 42448670375][run-queue] green on [b670378].

[7897e70]: https://github.com/victor-software-house/ctl-core/commit/7897e70c03967ffc3c96cfaf8bb1fb04f3576c55
[research]: ../../../../docs/research/shell-completion.md
[18b76f9]: https://github.com/victor-software-house/ctl-core/commit/18b76f94173226129cb2402a137eab32accfbbdb
[7cc88ad]: https://github.com/victor-software-house/ctl-core/commit/7cc88ad533f286fbedd157d67d7254c948a52fbf
[run]: https://github.com/victor-software-house/ctl-core/actions/runs/42431161985
[script-snapshots]: ../2026-11-08-script-snapshots/proposal.md
[#180]: https://github.com/victor-software-house/ctl-core/pull/180
[#188]: https://github.com/victor-software-house/ctl-core/pull/188
[a1a0fcd]: https://github.com/victor-software-house/ctl-core/commit/a1a0fcd4551c056069d2f77e70137f55512e43ec
[e02e9e6]: https://github.com/victor-software-house/ctl-core/commit/e02e9e6c96f96efae934ad26e2a59a7f271884ea
[b670378]: https://github.com/victor-software-house/ctl-core/commit/b6703788d2ebb1a04113bc1cc00f4901cb320951
[run-180]: https://github.com/victor-software-house/ctl-core/actions/runs/42447333133
[run-188]: https://github.com/victor-software-house/ctl-core/actions/runs/42447452161
[run-queue]: https://github.com/victor-software-house/ctl-core/actions/runs/42448670375

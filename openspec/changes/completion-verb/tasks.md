# Tasks

- [x] 1. Research how current CLIs ship completion, and commit it as
  [`docs/research/shell-completion.md`][research]. Proof: the markdown lint
  passes on the file.
  - 2026-11-08 UTC: committed as [7897e70]; `rumdl check` reported no
    issues.
- [ ] 2. Plan: this change, and the CTC-030 row added with `qctl add`.
  Proof: `openspec validate completion-verb --strict` and
  `mise run q check` pass.
- [ ] 3. Add `Shell` in `src/shell.rs` behind the `shell` feature. Proof:
  the parse scenarios in `tests/completion.rs` pass in CI.
- [ ] 4. Add `CompletionCommand`, `Shell::script`, `Script`, and `view` in
  `src/completion.rs` behind the `completion` feature, with
  `clap_complete` pinned to `=4.6.11`. Proof: the script, help, and view
  tests in `tests/completion.rs` pass in CI.
- [ ] 5. Add `App::completion()`. Proof: the environment tests in
  `tests/completion.rs`, each in a child process with `COMPLETE` set, pass
  in CI.
- [ ] 6. Add `shell` and `completion` to `check:features`, document both
  features in the crate docs and `AGENTS.md`, and add the changeset.
  Proof: `mise run verify` passes in CI on the pushed commit.
- [ ] 7. Mark the PR ready, wait for CI and Kody, and answer each thread.
  Proof: CI green on the last push.

[7897e70]: https://github.com/victor-software-house/ctl-core/commit/7897e70c03967ffc3c96cfaf8bb1fb04f3576c55
[research]: ../../../docs/research/shell-completion.md

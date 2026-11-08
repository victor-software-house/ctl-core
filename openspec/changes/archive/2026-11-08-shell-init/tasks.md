# Tasks

- [x] 1. Plan: the research narrowed to one first-prompt hook, this change,
  and the CTC-033 row.
  Proof: `openspec validate shell-init --strict` and `mise run q check`
  pass.
  - 2026-11-08 UTC: CTC-033 added with `qctl add`; `openspec validate --all
    --strict`, `mise run q check`, and `mise run check:repo` passed.
- [x] 2. Prove a pseudo-terminal drives bash, zsh, and fish through their
  prompts.
  Proof: a prototype on `pty-process` 0.5.4 runs the drafted hook once in
  each shell, and not at all without the variable or the binary.
  - 2026-11-08 UTC: on a macOS arm64 workstation with conda zsh 5.9.2 and
    conda fish 4.9.3, the versions `mise.toml` pins, all nine cases passed.
- [x] 3. Add `shell_init` behind the `shell-init` feature, and move
  `Script` and `view` to `shell`.
  Proof: `check:features` builds `shell,view`, `shell-init`, and
  `app,shell-init` in CI.
  - 2026-11-08 UTC: landed in [#191]; CI [run 42451994338][run-191] built
    all three sets on its last push, [20b02d5]. `shell,view` and `shell-init`
    printed dead-code warnings, so [#194] gated `parsed_output` on `help` and made
    `check:features` deny warnings; its CI [run 42451996008][run-194]
    passed on [f0831cb].
- [x] 4. Add the snapshot, shell-check, and terminal tests.
  Proof: `ctl-core/tests/shell_init.rs` passes with `INSTA_UPDATE=no` in CI.
  - 2026-11-08 UTC: on a macOS arm64 workstation with bash 5.3, all 12
    tests passed, the snapshots were written and reviewed, and clippy on
    all targets passed with `-D warnings`. CI on linux-x64 has not run.
  - 2026-11-08 UTC: two review fixes added
    `no_shell_takes_a_function_for_the_binary` and
    `bash_runs_the_hook_once_after_bash_preexec_rewrites_the_prompt_command`.
    The file passed in CI [run 42451994338][run-191] on [20b02d5] on
    linux-x64.
- [x] 5. Update `README.md`, `AGENTS.md`, the `completion-verb` change, and
  add the changeset.
  Proof: `mise run verify` passes in CI on the pushed commit.
  - 2026-11-08 UTC: in [#191], with `.changeset/shell-init.md`;
    `mise run verify` passed in CI [run 42451994338][run-191] on
    [20b02d5].
- [x] 6. Open the PR ready on the stack top, wait for CI and Kody, and
  answer each thread in the top PR before the stack merges.
  Proof: CI green on the last push.
  - 2026-11-08 UTC: CI passed on the last push to [#191] and to [#194], as
    tasks 3 and 4 record. Kody's two threads on [#191], the `PATH`-only
    lookups and the run-once guard, were fixed in [#191] and are resolved;
    [#194] drew none. The stack merged through the merge queue, with CI
    [run 42452511771][run-queue] green on [d370c8e].

[#191]: https://github.com/victor-software-house/ctl-core/pull/191
[#194]: https://github.com/victor-software-house/ctl-core/pull/194
[20b02d5]: https://github.com/victor-software-house/ctl-core/commit/20b02d53e273f708f3a6e463e2f7536e45465110
[f0831cb]: https://github.com/victor-software-house/ctl-core/commit/f0831cb6ee68376d783b68dde19a7927d97f02db
[d370c8e]: https://github.com/victor-software-house/ctl-core/commit/d370c8e1290ee019a761f4ff76904a8ea163eb4a
[run-191]: https://github.com/victor-software-house/ctl-core/actions/runs/42451994338
[run-194]: https://github.com/victor-software-house/ctl-core/actions/runs/42451996008
[run-queue]: https://github.com/victor-software-house/ctl-core/actions/runs/42452511771

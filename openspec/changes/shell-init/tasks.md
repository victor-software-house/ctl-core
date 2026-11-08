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
- [ ] 3. Add `shell_init` behind the `shell-init` feature, and move
  `Script` and `view` to `shell`.
  Proof: `check:features` builds `shell,view`, `shell-init`, and
  `app,shell-init` in CI.
- [ ] 4. Add the snapshot, shell-check, and terminal tests.
  Proof: `ctl-core/tests/shell_init.rs` passes with `INSTA_UPDATE=no` in CI.
  - 2026-11-08 UTC: on a macOS arm64 workstation with bash 5.3, all 12
    tests passed, the snapshots were written and reviewed, and clippy on
    all targets passed with `-D warnings`. CI on linux-x64 has not run.
- [ ] 5. Update `README.md`, `AGENTS.md`, the `completion-verb` change, and
  add the changeset.
  Proof: `mise run verify` passes in CI on the pushed commit.
- [ ] 6. Open the PR ready on the stack top, wait for CI and Kody, and
  answer each thread in the top PR before the stack merges.
  Proof: CI green on the last push.

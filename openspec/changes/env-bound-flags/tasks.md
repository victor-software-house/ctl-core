# Tasks

- [x] 1. Plan: this change, and the CTC-022 row added with `qctl add`. Proof:
  `openspec validate env-bound-flags --strict` and `mise run q check` pass.
- [ ] 2. Add `EnvPrefix` in `src/flags/env.rs`, and export it
  from `flags`, the crate root, and the prelude. Proof: the unit tests in
  `src/flags/env.rs` pass on a build runner.
- [ ] 3. Print `[env: NAME]` in ctl-core help. Proof:
  `help_names_each_variable` in `tests/env_flags.rs` passes.
- [ ] 4. Add `env_under_flags` in `src/flags/env.rs`, export it, and call it
  in `App` before the parse. Proof: the negation, conflict, group,
  exclusive, false-variable, and two-variable scenarios in
  `tests/env_flags.rs` pass.
- [ ] 5. Prove the parse through the environment in a child process. Proof:
  every test in `tests/env_flags.rs` passes, and `mise run verify` passes on
  a build runner.
- [ ] 6. Document the binding and the precedence rule in AGENTS.md, and add
  the changeset. Proof: the pre-push `verify` passes on the pushed commit.
- [ ] 7. Open the PR, wait for CI and Kody, and answer each thread with the
  commit that fixes it. Proof: CI green on the last push, no open thread.

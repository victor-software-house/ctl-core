# Tasks

- [x] 1. Plan: this change, and the CTC-032 row added with `qctl add`.
  Proof: `openspec validate narrow-clap-features --strict` and
  `mise run q check` pass.
- [ ] 2. Declare `clap` with `default-features = false`; `cli` asks for
  `std`, `derive`, `env`, and `string`, and `help` adds `clap/usage`.
  Proof: `check:features` passes in CI for every advertised set.
- [ ] 3. Render the help Clap stops for in `App::clap_error`, and remove
  the `Command::color` calls. Proof: `tests/clap_display.rs` passes in CI.
- [ ] 4. Add `ctl_core::verbatim` behind the `verbatim` feature, call it
  from the renderer, and drop `unicode-general-category`. Proof: the
  predicate tests in `src/verbatim.rs` and the verbatim render tests pass
  in CI.
- [ ] 5. Measure the toy CLI's size per tier on the commit CI verified.
  Proof: the table below, with the commit and the CI run.
- [ ] 6. Open the PR ready on the stack top, wait for CI and Kody, and
  answer each thread. Proof: CI green on the last push.

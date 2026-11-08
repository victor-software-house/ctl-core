# Tasks

- [x] 1. Plan: this change, and the CTC-032 row added with `qctl add`.
  Proof: `openspec validate narrow-clap-features --strict` and
  `mise run q check` pass.
- [x] 2. Declare `clap` with `default-features = false`; `cli` asks for
  `std`, `derive`, `env`, and `string`, and `help` adds `clap/usage`.
  Proof: `check:features` passes in CI for every advertised set.
  - 2026-11-08 UTC: landed in [#183]. Its last push, [c38232f], failed
    Build and test in [run 42447333385][run-183] before any cargo step:
    mise found no linux-x64 lock entry for a conda dependency of zsh and
    fish, the defect [#187] fixed. `check:features` runs inside that job,
    so it did not run there. It passed on the PRs above, in CI
    [run 42447333033][run-186] on [ed01cac] and [run 42447332218][run-187]
    on [5f320ff], and in the merge queue, [run 42448670375][run-queue] on
    [b670378].
- [x] 3. Render the help Clap stops for in `App::clap_error`, and remove
  the `Command::color` calls. Proof: `tests/clap_display.rs` passes in CI.
  - 2026-11-08 UTC: the code landed in [#183], and [#186] snapshot the help
    a missing argument prints. `tests/clap_display.rs` passed in CI
    [run 42447333033][run-186] on [ed01cac], the last push to [#186], and
    in the merge queue on [b670378].
- [x] 4. Add `ctl_core::verbatim` behind the `verbatim` feature, call it
  from the renderer, and drop `unicode-general-category`. Proof: the
  predicate tests in `src/verbatim.rs` and the verbatim render tests pass
  in CI.
  - 2026-11-08 UTC: landed in [#183]; the tests passed in the same runs as
    task 2. `unicode-general-category` is gone from `Cargo.lock`. The
    predicate later moved to the `ctl-text` crate in [#193], and the
    `verbatim` feature now enables it.
- [x] 5. Measure the toy CLI's size per tier on the commit CI verified.
  Proof: the table below, with the commit and the CI run.
  - 2026-11-08 UTC: measured on [b670378], which CI
    [run 42448670375][run-queue] verified. These are local measurements
    on a macOS workstation, not CI output. The setup is the one under
    [Measurements](design.md#measurements): rustc 1.99.1, darwin-arm64,
    opt-level `"s"`, fat LTO, `codegen-units = 1`, `panic = "abort"`,
    `strip = true`; the toy CLI on Clap 4.6.7 with
    `default-features = false` and `["std", "derive"]`, and ctl-core with
    `default-features = false`. `nm` lists only the dynamic imports and
    one symbol, so the strip ran.

  | Tier | 0.6.28 file / `__text` | [b670378] file / `__text` | `__text` change |
  |:--|--:|--:|--:|
  | `cli` | 468,624 / 361,224 | 402,256 / 299,208 | −17.2% |
  | `help` | 685,760 / 476,592 | 586,528 / 427,712 | −10.3% |
  | `view` | 669,232 / 462,440 | 553,424 / 401,908 | −13.1% |
  | `app` | 718,864 / 498,948 | 619,664 / 450,192 | −9.8% |

- [x] 6. Open the PR ready on the stack top, wait for CI and Kody, and
  answer each thread. Proof: CI green on the last push.
  - 2026-11-08 UTC: [#183]'s last push failed CI, as task 2 records; the
    green runs above it carry its commits. Kody left no thread on [#183].
    Its one thread on [#186] was fixed on the stack top in [#188], whose
    CI [run 42447452161][run-188] passed on [e02e9e6], and is resolved.
    [#194] later gated `parsed_output` on `help` and made
    `check:features` deny warnings; its CI [run 42451996008][run-194]
    passed on [f0831cb].

[#183]: https://github.com/victor-software-house/ctl-core/pull/183
[#186]: https://github.com/victor-software-house/ctl-core/pull/186
[#187]: https://github.com/victor-software-house/ctl-core/pull/187
[#188]: https://github.com/victor-software-house/ctl-core/pull/188
[#193]: https://github.com/victor-software-house/ctl-core/pull/193
[#194]: https://github.com/victor-software-house/ctl-core/pull/194
[c38232f]: https://github.com/victor-software-house/ctl-core/commit/c38232f42f69c3a89ff08fa3d9225690662a095f
[ed01cac]: https://github.com/victor-software-house/ctl-core/commit/ed01cac0f56ff79191c3b8e81afa50d1571f60f0
[5f320ff]: https://github.com/victor-software-house/ctl-core/commit/5f320ff41fe10f0e3227f55a458b389c493afe07
[e02e9e6]: https://github.com/victor-software-house/ctl-core/commit/e02e9e6c96f96efae934ad26e2a59a7f271884ea
[b670378]: https://github.com/victor-software-house/ctl-core/commit/b6703788d2ebb1a04113bc1cc00f4901cb320951
[f0831cb]: https://github.com/victor-software-house/ctl-core/commit/f0831cb6ee68376d783b68dde19a7927d97f02db
[run-183]: https://github.com/victor-software-house/ctl-core/actions/runs/42447333385
[run-186]: https://github.com/victor-software-house/ctl-core/actions/runs/42447333033
[run-187]: https://github.com/victor-software-house/ctl-core/actions/runs/42447332218
[run-188]: https://github.com/victor-software-house/ctl-core/actions/runs/42447452161
[run-194]: https://github.com/victor-software-house/ctl-core/actions/runs/42451996008
[run-queue]: https://github.com/victor-software-house/ctl-core/actions/runs/42448670375

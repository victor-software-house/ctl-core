# Tasks

Compile, format, lint and test tasks are proved by CI on the pushed commit,
never on the authoring workstation. Measurements run on a darwin-arm64 host
and in CI, as each task says.

## 1. Plan

- [x] 1.1 Commit the research in `docs/research/terminal-rendering/`, this
  change, and the CTC-034 row with the later phases as horizon rows. Proof:
  `openspec validate --all --strict` passes on the pushed commit, and
  `mise run q check` passes on main.
  - Amended 2026-11-08 UTC in [89f907a]. The proof first read: "`openspec
    validate --all --strict` and `mise run q check` pass on the pushed
    commit, and the maintainer approves the decisions listed in the
    proposal." On the pushed commit of [#185], `q check` reported CTC-030
    to CTC-033 as missing, because the pull requests that add those rows
    were still open, so the check now runs on main. [#185] carries no
    formal review, so the approval clause is dropped. The sign-off on this
    amendment is the maintainer's merge of [#222], which carries it; that
    pull request has no formal review either.
  - 2026-11-08 UTC: committed in [#185], which the maintainer merged. CI
    [run 42439929592][run-185] passed on [2cc125b], and the merge queue's
    [run 42440121205][q-185] on [e0b5249]. `openspec validate --all
    --strict` passed, 7 of 7. On that commit, `mise run q check` reported
    CTC-030 to CTC-033 as missing ids, because their pull requests were
    still open. On main after [#198] landed those rows, `mise run q check`
    passes, and `openspec validate --all --strict` passes 11 of 11.

## 2. Workspace

- [x] 2.1 Move the crate to `ctl-core/` under a virtual root manifest
  with `resolver = "3"`, workspace version, lints and dependencies, through a
  copier update with `layout: workspace`. Proof: CI passes with no change to
  any test, and `cargo package --workspace --locked` lists `ctl-core` at the
  same version.
  - 2026-11-08 UTC: [#192]. CI [run 42449370902][run-192] passed on
    [6b01720], and the merge queue's [run 42449643971][q-192] on [0765c15].
    The test files moved to `ctl-core/tests/` unchanged (`git diff -M`
    shows 0 changed lines in them). The run's Package step packaged and
    verified `ctl-core v0.6.28`, the version before the move.
- [x] 2.2 Point `.ctl/ver.yaml` at the virtual manifest so verctl releases the
  workspace as one package. Proof: `verctl publish --dry-run` lists the
  members in dependency order, and `mise run deny` passes in CI.
  - Amended 2026-11-08 UTC. The task first named `verctl publish --plan`,
    which now lists only version commits whose release is incomplete;
    `--dry-run` prints the publish order. It also asked for `wrappers`
    entries in `deny.toml`, which allow a named crate to depend on a
    banned one. `deny.toml` bans only `openssl` and `native-tls`, which no
    member uses, so no ban needs a wrapper and none was added. The
    sign-off is the maintainer's merge of [#221], which carries this
    amendment in [253c007]; it has no formal review.
  - 2026-11-08 UTC: `.ctl/ver.yaml` reads the root `Cargo.toml`, the
    virtual manifest, and pins each member's `={version}` entry in
    `[workspace.dependencies]`. On main at [93c8642],
    `verctl publish --dry-run`, run through `mise run ver`, listed
    `ctl-text`, `ctl-layout`, `ctl-term`, then `ctl-core`, all at 0.6.28
    through cargo, and published nothing. `mise run deny` runs
    `cargo deny --locked check licenses bans sources`, which passed in the
    merge queue's [run 42460567403][q-213].

## 3. `ctl-text`

- [x] 3.1 Add `ctl-text/` with `Span`, `Line`, `Text`, the cell-width
  functions, `overlay`, and `UNICODE_VERSION`; move CTC-032's `verbatim`
  module into it. Proof: unit tests for the width scenarios of the
  `styled-text` spec and the version pin test pass in CI.
  - 2026-11-08 UTC: [#193]. CI [run 42451537424][run-193] passed on
    [fcd8f62], and the merge queue's [run 42451743356][q-193] on [ea5ab25],
    including `tests/it/styled_text.rs` and `tests/it/unicode_version.rs`.
- [x] 3.2 Add wrapping with keep-words and fold, truncation with an ellipsis,
  tab expansion and trailing-space removal. Proof: the `styled-text`
  scenarios and the property tests of design decision 17 pass in CI.
  - 2026-11-08 UTC: [#193], in the same runs as 3.1, including the proptest
    properties in `tests/it/properties.rs`.
- [x] 3.3 Add `Renderable`, `Measurement` and `Context`, and implement
  `Renderable` for `Text`. Proof: a doc-test implements a widget as in design
  decision 15 and passes in CI; the crate root docs carry a quick start.
  - 2026-11-08 UTC: [#193], in the same runs as 3.1. The crate root's
    `# Quick start` implements `Renderable` for a `Meter` widget as a
    doc-test, and `test:doc` ran it.

## 4. `ctl-layout`

- [x] 4.1 Add `ctl-layout/` with `Table` and its five-step column
  algorithm, per-column overflow and numeric alignment, the frames ctl-core
  uses today (grid, header rule, plain, row rule, blank rows), and stacking.
  Proof: the `layout` spec scenarios, insta snapshots at widths 20, 40, 80 and
  120, and the width properties pass in CI.
  - 2026-11-08 UTC: [#197]. CI [run 42452982117][run-197] passed on
    [e2df8f6], and the merge queue's [run 42453256063][q-197] on [eb81419].
    `ctl-layout/tests/snapshots/` holds 20 widgets at each of the four
    widths.
- [x] 4.2 Add `Record`, `Rule`, `Stack` and `Indent`. Proof: insta snapshots
  and the width properties pass in CI; rustdoc of every public item has an
  example or a one-line summary.
  - 2026-11-08 UTC: [#197], in the same runs as 4.1. `missing_docs` is
    denied, so every public item carries at least a summary.

## 5. `ctl-term`

- [x] 5.1 Add `ctl-term/` with `Terminal::detect` over an environment
  lookup, colour depth, and the nearest-colour conversion. Proof: a table
  test over every variable in design decision 10, alone and in conflicting
  pairs, passes in CI.
  - 2026-11-08 UTC: [#199], with [#200] and [#202] fixing its closed-pipe
    test. CI [run 42455275847][run-202] passed on [9d56366], and the merge
    queue's [run 42455512195][q-199] on [82958db]. The detection tests
    cover 31 colour environments on a pipe and on a terminal, `Always` and
    `Never` against all 31, 18 depth cases and 31 width cases.
- [x] 5.2 Add the encoder and `Console`: style changes only, a reset at each
  line end, plain bytes when colour is off, width per stream through
  `terminal_size`, virtual-terminal mode on Windows, and `BrokenPipe`
  reported as its own error. Proof: the `terminal-output` scenarios pass in
  CI, the closed-pipe one through a child process piped to `head -1`.
  - 2026-11-08 UTC: [#199], [#200] and [#202], in the same runs as 5.1.
    `tests/it/scenarios.rs` holds the scenarios, and the closed-pipe test
    runs the test binary as a child piped to `head -1` on Linux. Not
    verified: the Windows virtual-terminal path, since CI has no Windows
    job, and the closed-pipe test on macOS.

## 6. ctl-core on the engine

- [x] 6.1 Map `Document` to `ctl-layout` widgets through `Theme`, add
  `ellipsis_column` and `numeric_column`, and write `View`, help and errors
  through `Console`. Proof: the existing ctl-core tests pass in CI; the
  `presentation` delta scenarios pass, the stderr help one in a child process.
  - 2026-11-08 UTC: [#206], with [#207], [#208] and [#210]. CI
    [run 42458776026][run-210] passed on [e06097e], and the merge queue's
    [run 42458984812][q-209] on [c17236d], including the help snapshots
    rendered in a child process and the pty test on Linux.
- [x] 6.2 Add the parity test: the consumer corpus rendered by ctl-core 0.6.x
  and by the engine, both committed, every difference listed with its reason
  in the pull request. Proof: the parity test passes in CI and the pull
  request lists each difference.
  - 2026-11-08 UTC: [#203] committed the 0.6.x baselines and [#204] fixed
    its lint; CI [run 42456492298][run-204] passed on [5b31f40], and the
    merge queue's [run 42456761995][q-203] on [39e6f88]. [#206] added the
    engine renders and [`ctl-core/tests/parity/DIFFERENCES.md`][differences],
    which lists each of the eight differences with an example and its
    reason; [#206]'s description summarises them. [#208] changed
    difference 5 and updated that file. The parity test passed in
    [run 42458984812][q-209]. The committed file now carries the
    per-difference list the proof asked of the pull request. The sign-off
    on that substitution is the maintainer's merge of [#223], which carries
    this sentence in [ee51e4b]; it has no formal review.
- [x] 6.3 Remove comfy-table, anstream, unicode-general-category, `kv` and
  `grid`; add `App::broken_pipe` and `App::theme`. Proof: `cargo tree` in CI
  lists none of comfy-table, crossterm, anstream, anstyle-parse or console
  for `ctl-core` with `app`; the colour test with `CLICOLOR_FORCE=1` finds no
  ESC in JSON.
  - 2026-11-08 UTC: [#206] removed the crates; [#211] removed `kv` and
    `grid` and added `App::broken_pipe`, `App::theme` and `check:tree`.
    [#214], [#215] and [#216] fixed the closed-pipe exit codes and a lint.
    CI [run 42460354659][run-216] passed on [6b2be38], and the merge
    queue's [run 42460567403][q-213] on [93c8642]. Both runs include
    `check:tree`, which runs `cargo tree` for `ctl-core` with `app` and
    fails on any of the five crates, and
    `a_mounted_task_pipes_json_without_an_escape_byte`, which runs with
    `CLICOLOR_FORCE=1`.
- [x] 6.4 Replace the rule in `docs/presentation.md` with design decision 18,
  update `AGENTS.md`, `README.md`, crate docs and the feature list, and add
  the changeset, which states each break. Proof: `rg 'comfy|anstream'` over
  `docs`, `AGENTS.md` and `README.md` finds only history, or a rule that
  names the crates as removed, and the pre-push hooks pass.
  - Amended 2026-11-08 UTC. The proof first allowed only history. The
    `check:tree` task that 6.3 added must name the crates it refuses, and
    `AGENTS.md` describes that task, so a current rule naming them is
    allowed too. The sign-off is the maintainer's merge of [#221], which
    carries this amendment in [ae33a31]; it has no formal review.
  - 2026-11-08 UTC: [#212], in the same runs as 6.3; the pre-push hooks
    passed on its push. On this branch, `rg 'comfy|anstream'` over `docs`,
    `AGENTS.md` and `README.md` finds the research in
    `docs/research/terminal-rendering/`, which is history, and one
    `AGENTS.md` line listing the crates `check:tree` refuses: a rule that
    names the crates as removed, which the amended proof allows.

## 7. Size

- [x] 7.1 Add `check:size`: build the toy CLI at each tier in CI and compare
  `.text` with the `[workspace.metadata.size]` budgets in the root
  `Cargo.toml`. Proof: the task passes on the pushed commit and fails when
  a tier's budget is lowered by 5%.
  - Amended 2026-11-08 UTC. The proof first asked for a commit that adds 5%
    to a tier. A budget lowered by 5% makes the same comparison, `.text`
    against its limit, without changing the code the toy builds. The
    sign-off is the maintainer's merge of [#222], which carries this
    amendment in [c3f6883]; it has no formal review.
  - 2026-11-08 UTC: [#217] added the task and the budgets, and [#218] made a
    tier with no `.text` fail. CI [run 42460793731][run-217] passed on
    [bbb006b] and measured cli 343,888, help 443,676, view 409,362 and app
    469,842 bytes, the budgets. CI [run 42461005097][run-218] passed on
    [4035f78], and the merge queue's [run 42461303294][q-219] on
    [b7c1680]. On a linux-x64 host with rustc 1.99.1 and GNU size 2.46,
    with `MISE_ENV=ci`, the task passed with those budgets and failed on all
    four tiers with each budget lowered by 5%: cli measured 343,888 bytes
    against a limit of 333,226.
- [x] 7.2 Measure each tier and each public consumer at the commit CI verified,
  on darwin-arm64 with the release profile, and record file, `__text` and
  package counts against ctl-core 0.6.x in this change's design. Proof: the
  table is in the design with the commit and the strip check.
  - 2026-11-08 UTC: measured at [2f23725], which CI verified in
    [run 42542525587][ci-2f23725], on an Apple M4 Pro with rustc 1.99.1, in a
    scratch clone deleted afterwards; nothing was committed to a consumer.
    Design decision 16 holds the tier and consumer tables, the strip check
    (no Rust symbol in `nm -U` on any of the 20 binaries), and a cargo-bloat
    attribution of qctl and rawr. The toy's `__text` fell by 62,016 to
    101,224 bytes per tier. The consumers moved by -7,232 (verctl), -9,472
    (qctl), -7,104 (forkctl) and +2,644 (rawr). Not verified: the split of
    the toy's change between clap and the engine on `main`, which reuses
    Matrix B's clap figures.

## Release prerequisites

- The first publish of `ctl-text`, `ctl-layout`, and `ctl-term` needs a
  registry token. crates.io configures [trusted publishing] only on a crate
  that already exists, and the release workflow passes only
  `crates-io-trusted-publishing: true`.
  1. The release pull request passes the `CARGO_REGISTRY_TOKEN` repository
     secret as the shared publish workflow's `registry-token`, with trusted
     publishing off; that workflow refuses both at once.
  2. After the publish, the maintainer configures a GitHub trusted publisher
     on the three crates.
  3. A pull request turns trusted publishing back on.

  The maintainer owns these steps. Nothing changes before the release is
  approved.
- The release waits on a prose check in CI. `mise run lint:prose` runs Vale
  over the Markdown, the Rust comments, and the changesets, and the docs
  reach it with 0 findings. It stays out of `verify`, which runs only
  offline checks, because `vale sync` downloads the style package.
  1. The shared template's `lint_prose` piece adds the check when it ships.
  2. If that piece is not out when the rest of the release is ready, a pull
     request runs `lint:prose` as its own CI step, as `deny:advisories`
     runs, and `verify` stays offline.

## 8. Integration

- [ ] 8.1 After release, each public consumer (verctl, qctl, forkctl, rawr)
  moves its pin in its own pull request and reviews its snapshot diff. Proof:
  each pull request's CI passes, linked here.

## Workflow follow-up

- Archive the change after the release and the four consumer pull requests
  merge, then fold the specs into `openspec/specs/`.
- Open one change per later phase from its horizon row.

[#185]: https://github.com/victor-software-house/ctl-core/pull/185
[#192]: https://github.com/victor-software-house/ctl-core/pull/192
[#193]: https://github.com/victor-software-house/ctl-core/pull/193
[#197]: https://github.com/victor-software-house/ctl-core/pull/197
[#198]: https://github.com/victor-software-house/ctl-core/pull/198
[#199]: https://github.com/victor-software-house/ctl-core/pull/199
[#200]: https://github.com/victor-software-house/ctl-core/pull/200
[#202]: https://github.com/victor-software-house/ctl-core/pull/202
[#203]: https://github.com/victor-software-house/ctl-core/pull/203
[#204]: https://github.com/victor-software-house/ctl-core/pull/204
[#206]: https://github.com/victor-software-house/ctl-core/pull/206
[#207]: https://github.com/victor-software-house/ctl-core/pull/207
[#208]: https://github.com/victor-software-house/ctl-core/pull/208
[#210]: https://github.com/victor-software-house/ctl-core/pull/210
[#211]: https://github.com/victor-software-house/ctl-core/pull/211
[#212]: https://github.com/victor-software-house/ctl-core/pull/212
[#214]: https://github.com/victor-software-house/ctl-core/pull/214
[#215]: https://github.com/victor-software-house/ctl-core/pull/215
[#216]: https://github.com/victor-software-house/ctl-core/pull/216
[2cc125b]: https://github.com/victor-software-house/ctl-core/commit/2cc125b0b53b6bebcf421298883a59b06ab5fe36
[e0b5249]: https://github.com/victor-software-house/ctl-core/commit/e0b5249660cbec9fb5813341e15ac0daf0b3af4d
[6b01720]: https://github.com/victor-software-house/ctl-core/commit/6b01720b109c48f2c40dae3e87aa87df1db47970
[0765c15]: https://github.com/victor-software-house/ctl-core/commit/0765c15c7892abb9626f98b263ba2a22dd6426f9
[fcd8f62]: https://github.com/victor-software-house/ctl-core/commit/fcd8f62634ca185cb15c7d071913d7a37426e79e
[ea5ab25]: https://github.com/victor-software-house/ctl-core/commit/ea5ab25e105d91dff595384a00ed285d00eeb515
[e2df8f6]: https://github.com/victor-software-house/ctl-core/commit/e2df8f61cdd1b38a037d1377b614545b2b17998c
[eb81419]: https://github.com/victor-software-house/ctl-core/commit/eb814194a3aab22fdc5e8e9bb32df93fd37d6bc0
[9d56366]: https://github.com/victor-software-house/ctl-core/commit/9d5636619e8d37d1cdf8058240d62f47909c36d8
[82958db]: https://github.com/victor-software-house/ctl-core/commit/82958dba27e7b8bb22a9ebbddad4ff209037e6df
[5b31f40]: https://github.com/victor-software-house/ctl-core/commit/5b31f40b5723083bc3dbeebae116234a91272180
[39e6f88]: https://github.com/victor-software-house/ctl-core/commit/39e6f883a0bec3aa3f551db3ee76fc45ae771183
[e06097e]: https://github.com/victor-software-house/ctl-core/commit/e06097e09a17481bbdf292860ef2a1dd9dbd8ce5
[c17236d]: https://github.com/victor-software-house/ctl-core/commit/c17236dc26b72fb3d01546279c0e575cb3cb6853
[6b2be38]: https://github.com/victor-software-house/ctl-core/commit/6b2be38d958011b7f447a3a60e8b9877cdb278eb
[93c8642]: https://github.com/victor-software-house/ctl-core/commit/93c86420e13afdb7a65fb3beb272a32012713402
[run-185]: https://github.com/victor-software-house/ctl-core/actions/runs/42439929592
[q-185]: https://github.com/victor-software-house/ctl-core/actions/runs/42440121205
[run-192]: https://github.com/victor-software-house/ctl-core/actions/runs/42449370902
[q-192]: https://github.com/victor-software-house/ctl-core/actions/runs/42449643971
[run-193]: https://github.com/victor-software-house/ctl-core/actions/runs/42451537424
[q-193]: https://github.com/victor-software-house/ctl-core/actions/runs/42451743356
[run-197]: https://github.com/victor-software-house/ctl-core/actions/runs/42452982117
[q-197]: https://github.com/victor-software-house/ctl-core/actions/runs/42453256063
[run-202]: https://github.com/victor-software-house/ctl-core/actions/runs/42455275847
[q-199]: https://github.com/victor-software-house/ctl-core/actions/runs/42455512195
[run-204]: https://github.com/victor-software-house/ctl-core/actions/runs/42456492298
[q-203]: https://github.com/victor-software-house/ctl-core/actions/runs/42456761995
[run-210]: https://github.com/victor-software-house/ctl-core/actions/runs/42458776026
[q-209]: https://github.com/victor-software-house/ctl-core/actions/runs/42458984812
[run-216]: https://github.com/victor-software-house/ctl-core/actions/runs/42460354659
[q-213]: https://github.com/victor-software-house/ctl-core/actions/runs/42460567403
[differences]: https://github.com/victor-software-house/ctl-core/blob/b7c168096d8f5556ff59de6f31e64e8ab7a0ed8c/ctl-core/tests/parity/DIFFERENCES.md
[#217]: https://github.com/victor-software-house/ctl-core/pull/217
[#218]: https://github.com/victor-software-house/ctl-core/pull/218
[#221]: https://github.com/victor-software-house/ctl-core/pull/221
[#222]: https://github.com/victor-software-house/ctl-core/pull/222
[#223]: https://github.com/victor-software-house/ctl-core/pull/223
[89f907a]: https://github.com/victor-software-house/ctl-core/commit/89f907a4e12763af0490bf373dc827220dbf0201
[253c007]: https://github.com/victor-software-house/ctl-core/commit/253c007395405bd6c25f19a6c06fd24331a44bf2
[ae33a31]: https://github.com/victor-software-house/ctl-core/commit/ae33a31d466e89c06610a337751cd3f3da980d83
[c3f6883]: https://github.com/victor-software-house/ctl-core/commit/c3f6883b64d91e526bc06579446dd2e8636ece2b
[ee51e4b]: https://github.com/victor-software-house/ctl-core/commit/ee51e4b8d829ab8dd94a0887c7af5257ea89bb37
[bbb006b]: https://github.com/victor-software-house/ctl-core/commit/bbb006baf5611aefac85d34aeb5a276af109f78c
[4035f78]: https://github.com/victor-software-house/ctl-core/commit/4035f783456e71ae2b8099a47da5459f1e854786
[b7c1680]: https://github.com/victor-software-house/ctl-core/commit/b7c168096d8f5556ff59de6f31e64e8ab7a0ed8c
[run-217]: https://github.com/victor-software-house/ctl-core/actions/runs/42460793731
[run-218]: https://github.com/victor-software-house/ctl-core/actions/runs/42461005097
[q-219]: https://github.com/victor-software-house/ctl-core/actions/runs/42461303294
[trusted publishing]: https://crates.io/docs/trusted-publishing
[2f23725]: https://github.com/victor-software-house/ctl-core/commit/2f2372540862de53f1160ad247631a3ec1248ffb
[ci-2f23725]: https://github.com/victor-software-house/ctl-core/actions/runs/42542525587

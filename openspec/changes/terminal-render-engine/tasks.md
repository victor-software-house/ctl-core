# Tasks

Compile, format, lint and test tasks are proved by CI on the pushed commit,
never on the authoring workstation. Measurements run on a darwin-arm64 host
and in CI, as each task says.

## 1. Plan

- [ ] 1.1 Commit the research in `docs/research/terminal-rendering/`, this
  change, and the CTC-034 row with the later phases as horizon rows. Proof:
  `openspec validate --all --strict` and `mise run q check` pass on the pushed
  commit, and the maintainer approves the decisions listed in the proposal.

## 2. Workspace

- [ ] 2.1 Move the crate to `ctl-core/` under a virtual root manifest
  with `resolver = "3"`, workspace version, lints and dependencies, through a
  copier update with `layout: workspace`. Proof: CI passes with no change to
  any test, and `cargo package --workspace --locked` lists `ctl-core` at the
  same version.
- [ ] 2.2 Point `.ctl/ver.yaml` at the virtual manifest so verctl releases the
  workspace as one package, and add the `wrappers` entries for the new crates
  to `deny.toml`. Proof: `verctl publish --plan` lists the members in
  dependency order, and `mise run deny` passes in CI.

## 3. `ctl-text`

- [ ] 3.1 Add `ctl-text/` with `Span`, `Line`, `Text`, the cell-width
  functions, `overlay`, and `UNICODE_VERSION`; move CTC-032's `verbatim`
  module into it. Proof: unit tests for the width scenarios of the
  `styled-text` spec and the version pin test pass in CI.
- [ ] 3.2 Add wrapping with keep-words and fold, truncation with an ellipsis,
  tab expansion and trailing-space removal. Proof: the `styled-text`
  scenarios and the property tests of design decision 17 pass in CI.
- [ ] 3.3 Add `Renderable`, `Measurement` and `Context`, and implement
  `Renderable` for `Text`. Proof: a doc-test implements a widget as in design
  decision 15 and passes in CI; the crate root docs carry a quick start.

## 4. `ctl-layout`

- [ ] 4.1 Add `ctl-layout/` with `Table` and its five-step column
  algorithm, per-column overflow and numeric alignment, the frames ctl-core
  uses today (grid, header rule, plain, row rule, blank rows), and stacking.
  Proof: the `layout` spec scenarios, insta snapshots at widths 20, 40, 80 and
  120, and the width properties pass in CI.
- [ ] 4.2 Add `Record`, `Rule`, `Stack` and `Indent`. Proof: insta snapshots
  and the width properties pass in CI; rustdoc of every public item has an
  example or a one-line summary.

## 5. `ctl-term`

- [ ] 5.1 Add `ctl-term/` with `Terminal::detect` over an environment
  lookup, colour depth, and the nearest-colour conversion. Proof: a table
  test over every variable in design decision 10, alone and in conflicting
  pairs, passes in CI.
- [ ] 5.2 Add the encoder and `Console`: style changes only, a reset at each
  line end, plain bytes when colour is off, width per stream through
  `terminal_size`, virtual-terminal mode on Windows, and `BrokenPipe`
  reported as its own error. Proof: the `terminal-output` scenarios pass in
  CI, the closed-pipe one through a child process piped to `head -1`.

## 6. ctl-core on the engine

- [ ] 6.1 Map `Document` to `ctl-layout` widgets through `Theme`, add
  `ellipsis_column` and `numeric_column`, and write `View`, help and errors
  through `Console`. Proof: the existing ctl-core tests pass in CI; the
  `presentation` delta scenarios pass, the stderr help one in a child process.
- [ ] 6.2 Add the parity test: the consumer corpus rendered by ctl-core 0.6.x
  and by the engine, both committed, every difference listed with its reason
  in the pull request. Proof: the parity test passes in CI and the pull
  request lists each difference.
- [ ] 6.3 Remove comfy-table, anstream, unicode-general-category, `kv` and
  `grid`; add `App::broken_pipe` and `App::theme`. Proof: `cargo tree` in CI
  lists none of comfy-table, crossterm, anstream, anstyle-parse or console
  for `ctl-core` with `app`; the colour test with `CLICOLOR_FORCE=1` finds no
  ESC in JSON.
- [ ] 6.4 Replace the rule in `docs/presentation.md` with design decision 18,
  update `AGENTS.md`, `README.md`, crate docs and the feature list, and add
  the changeset, which states each break. Proof: `rg 'comfy|anstream'` over `docs`,
  `AGENTS.md` and `README.md` finds only history, and the pre-push hooks pass.

## 7. Size

- [ ] 7.1 Add `check:size`: build the toy CLI at each tier in CI and compare
  `.text` with the `[workspace.metadata.size]` budgets in the root
  `Cargo.toml`. Proof: the task passes on the
  pushed commit and fails on a commit that adds 5% to a tier.
- [ ] 7.2 Measure each tier and each public consumer at the commit CI verified,
  on darwin-arm64 with the release profile, and record file, `__text` and
  package counts against ctl-core 0.6.x in this change's design. Proof: the
  table is in the design with the commit and the strip check.

## 8. Integration

- [ ] 8.1 After release, each public consumer (verctl, qctl, forkctl, rawr)
  moves its pin in its own pull request and reviews its snapshot diff. Proof:
  each pull request's CI passes, linked here.

## Workflow follow-up

- Archive the change after the release and the four consumer pull requests
  merge, then fold the specs into `openspec/specs/`.
- Open one change per later phase from its horizon row.

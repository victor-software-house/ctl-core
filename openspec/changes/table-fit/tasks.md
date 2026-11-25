# Tasks

## 1. Floors

- [x] 1.1 Give each column a floor in ctl-layout and make the solver's limit the larger of the floor and the overflow limit.
  Verify: `widths.rs` and `table.rs` tests for the scenarios of the `layout` delta pass in CI.
  Evidence, 2026-11-25 (UTC): [PR 324][ctl-core#324] merged as `79a248a` with every check green in [its CI run][run-324].
  The `widths.rs` unit tests and `ctl-layout/tests/it/scenarios.rs` cover each scenario of the `layout` delta.
- [x] 1.2 Add `RenderOptions::column_floor`, its `View` and `App` setters, `CTL_CORE_COLUMN_FLOOR`, and `column_floor_envs`.
  Verify: tests for the environment, the override, 0, and an unaccepted value.
  Evidence, 2026-11-25 (UTC): [PR 324][ctl-core#324] merged as `79a248a` with every check green in [its CI run][run-324].
  `tests/table_fit.rs` and the `render.rs` unit tests cover the variable, an override, `0`, and an unaccepted value.
  `View` and `App` carry the floor through their existing `styles` setters, so no separate setter was added.
- [x] 1.3 Describe the column floor and `CTL_CORE_COLUMN_FLOOR` in `docs/presentation.md` and the rustdoc of `RenderOptions::column_floor`.
  Verify: `mise run lint:prose` and `mise run lint:md` pass locally, and the doc tasks pass in CI.
  Evidence, 2026-11-25 (UTC): `mise run lint:prose` (no error) and `mise run lint:md` pass on `main` at `79a248a`, and the doc tasks passed in [the CI run of PR 324][run-324].

## 2. Stacking by fit

- [x] 2.1 Add `Table::fits` in ctl-layout and a widget that draws the grid or the stacked records by fit; render the record form of design decision 2.
  Verify: snapshot tests for a six-column list at widths 40, 80, and 160.
  Evidence, 2026-11-25 (UTC): [PR 324][ctl-core#324] merged as `79a248a` with every check green in [its CI run][run-324].
  `a_six_column_list_at_each_width_and_style` snapshots a six-column list at widths 40, 80 and 160 in each list style.
- [x] 2.2 Add `TableFit`, `RenderOptions::table_fit`, `Table::stacking`, `CTL_CORE_TABLE_FIT`, and `table_fit_envs`; keep `stacked_below` as it is.
  Verify: the help snapshots do not change, and tests cover `wrap` and the warning.
  Evidence, 2026-11-25 (UTC): [PR 324][ctl-core#324] merged as `79a248a` with every check green in [its CI run][run-324].
  No help, usage or help parity snapshot moved, and `tests/table_fit.rs` covers `wrap` and the warning for an unaccepted value.
- [x] 2.3 Describe stacking by fit and `CTL_CORE_TABLE_FIT` in `docs/presentation.md` and the rustdoc of `Table` in both crates.
  Verify: `mise run lint:prose` and `mise run lint:md` pass locally, and the doc tasks pass in CI.
  Evidence, 2026-11-25 (UTC): `docs/presentation.md` and the rustdoc of `Table` in both crates describe stacking by fit; `lint:prose` and `lint:md` pass as under 1.3.

## 3. Release

- [x] 3.1 Write a `ctl-core: minor` changeset naming the rule and both variables.
  Verify: `openspec validate table-fit --strict` passes.
  Evidence, 2026-11-25 (UTC): `.changeset/table-fit.md` lists each break, the migration and both variables, and `openspec validate table-fit --strict` passes.

[ctl-core#324]: https://github.com/victor-software-house/ctl-core/pull/324
[run-324]: https://github.com/victor-software-house/ctl-core/actions/runs/45722916723

# Tasks

## 1. Floors

- [ ] 1.1 Give each column a floor in ctl-layout and make the solver's limit the larger of the floor and the overflow limit.
  Verify: `widths.rs` and `table.rs` tests for the scenarios of the `layout` delta pass in CI.
- [ ] 1.2 Add `RenderOptions::column_floor`, its `View` and `App` setters, `CTL_CORE_COLUMN_FLOOR`, and `column_floor_envs`.
  Verify: tests for the environment, the override, 0, and an unaccepted value.

## 2. Stacking by fit

- [ ] 2.1 Add `Table::fits` in ctl-layout and a widget that draws the grid or the stacked records by fit; render the record form of design decision 2.
  Verify: snapshot tests for a six-column list at widths 40, 80, and 160.
- [ ] 2.2 Add `TableFit`, `RenderOptions::table_fit`, `Table::stacking`, `CTL_CORE_TABLE_FIT`, and `table_fit_envs`; keep `stacked_below` as it is.
  Verify: the help snapshots do not change, and tests cover `wrap` and the warning.

## 3. Docs and release

- [ ] 3.1 Update `docs/presentation.md` and the rustdoc of `Table` in both crates.
  Verify: `mise run lint:prose` and the doc tasks pass in CI.
- [ ] 3.2 Write a `ctl-core: minor` changeset naming the rule and both variables.
  Verify: `openspec validate table-fit --strict` passes.

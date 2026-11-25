---
ctl-core: minor
---

No column of a table narrows below its floor, and a table that cannot fit at its floors stacks.
A column's floor is its header width or 8 cells, whichever is larger, and never more than its natural width.
A fold column, and a keep-words column in a grid, went down to its widest cluster before, and an ellipsis column to 1 cell.
Each now stops at its floor.
A table whose floors and frame do not fit the width draws each row as a record, with a blank line between rows.
A record is the first cell as a title, then a header and value line for each other column.
A table with no header draws each row as its first cell over the others, and a table with no rows draws its grid.

`RenderOptions::column_floor(cells)` sets the floor, and `CTL_CORE_COLUMN_FLOOR` sets it when the consumer leaves it open.
`0` turns the floor off, so each column goes down to the limit its overflow sets.

`RenderOptions::table_fit(TableFit)` sets the fit, and `Table::stacking(TableFit)` sets it for one table.
`CTL_CORE_TABLE_FIT` (`stack` or `wrap`) sets it when neither does.
`TableFit::Wrap` keeps the grid at its floors and lets it run past the width.
`column_floor_envs` and `table_fit_envs` replace each ordered name list, and an empty slice disables lookup.

A table with `Table::stacked_below`, such as a help table, has no floor and keeps its threshold and form, so help output does not change.

Public additions:

- ctl-core: `RenderOptions::column_floor`, `column_floor_envs`, `table_fit`, `table_fit_envs`, `resolved_column_floor`, and `resolved_table_fit`.
- ctl-core: `DEFAULT_COLUMN_FLOOR_ENV`, `DEFAULT_COLUMN_FLOOR_ENVS`, `DEFAULT_TABLE_FIT_ENV`, and `DEFAULT_TABLE_FIT_ENVS`.
- ctl-core: `TableFit`, at `ctl_core::TableFit`, `ctl_core::prelude::TableFit`, `ctl_core::document::TableFit`, and `ctl_core::render::TableFit`.
- ctl-core: `Table::stacking`, and its getter `Table::fit`, on the document `Table`.
- ctl-core: `View::styles` and `App::styles` carry the floor, the fit, and their name lists, as they carry the other styles.
- ctl-layout: `Table::column_floor`, `Table::fits`, and `Fallback`, which draws a table when it fits and another widget when it does not.

`RenderOptions` is larger, so a CLI function that takes it by value may meet clippy's pedantic `large_types_passed_by_value`.

Breaking changes:

1. Every table without `stacked_below` that cannot fit at its column limits now stacks, including unframed keep-words lists that used to run past the width.
2. No column narrows below its floor, so ellipsis and fold columns stop there, and a grid folds no column below it.
3. In ctl-layout, `Table`'s default column floor is 8, and its `measure` minimum rises with it.

Migration: set both `column_floor(0)` and `table_fit(TableFit::Wrap)`, or `CTL_CORE_COLUMN_FLOOR=0` and `CTL_CORE_TABLE_FIT=wrap`, to restore the old output.
A direct ctl-layout user calls `Table::column_floor(0)` and uses no `Fallback`.

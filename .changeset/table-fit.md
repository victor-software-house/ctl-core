---
ctl-core: minor
---

No column of a table narrows below its floor, and a table that cannot fit at its floors stacks.
A column's floor is its header width or 8 cells, whichever is larger, and never more than its natural width.
An ellipsis or fold column stops at its floor, where it went down to 1 cell before.
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
`ctl-layout` gains `Table::column_floor`, `Table::fits`, and `Fallback`, which draws a table when it fits and another widget when it does not.
A `ctl-layout` table has a floor of 8 cells unless it sets another, and a `Record` has none.

Breaking change: a table that relied on an ellipsis or fold column going below 8 cells now wraps at its floor or stacks.
Migration: set `column_floor(0)` or `table_fit(TableFit::Wrap)` to keep the old widths or the grid.

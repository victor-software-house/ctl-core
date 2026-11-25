# Design

## Context

[proposal.md](proposal.md) gives the reasons.
[widths.rs][widths] solves column widths in two passes.
It shrinks the widest columns down to each column's minimum, its widest word.
Then, in a framed table, it folds columns down to their fold limit (`Extent::floor` today), the widest grapheme cluster.
An ellipsis column has a minimum of 1 cell.

[render.rs][render] wraps a table in a `Breakpoint` only when the consumer set `stacked_below`.
Its stacked form is `Definitions`: label cells indented by 2, the rest below, indented by 4.

## Goals / Non-Goals

**Goals:**

1. A lower bound on every column, the same for every table.
2. A stacked form chosen by whether the table fits, not by a width the consumer guesses.

**Non-Goals:**

1. Dropping or hiding columns: the maintainer ruled that a table never crops (operator, 2026-11-24).
2. Copying a wrapped row whole, and an interactive table view, which are separate work.
3. A change to tables with a `stacked_below` threshold.

## Decisions

### 1. The floor is the header width or 8 cells

A column's floor is `min(natural, max(header width, floor))`, with the floor 8 cells by default; a floor setting of 0 sets no floor at all, header included, so the limits are the overflow limits.
The new bound is named the column floor in code, such as `Extent::column_floor`, so `Extent::floor`, the widest cluster, keeps its name and meaning.

1. The header bounds it from below, so a header never wraps mid-word.
2. 8 cells holds a short id, a date's `2026-11`, or two words of a title; a narrower column holds fragments.
3. The natural width bounds it from above, so a short column is never padded.

The solver's limit for each column becomes the larger of the floor and its overflow limit.
The overflow limit is the widest word for a keep-words column, the widest cluster for a fold column, and 1 cell for an ellipsis column.

Both passes of the solver use the new limit: the shrink to the minimums, and a grid's fold pass.
So a grid still folds a word wider than its column, but never below the floor.

The alternative was a share of the width, such as a tenth of the terminal.
It lost because it depends on the column count, and on a wide terminal it reserves space a short column does not need.

### 2. A table that cannot fit at its floors stacks

`Table::fits(width)` in ctl-layout reports whether the column limits and the frame fit.
The renderer draws the grid when it fits and the stacked form when it does not.
A widget chooses by fit, as `Breakpoint` chooses by width.

The stacked form of an automatic stack is one record per row.
The first column's value is the record's title, then one `header  value` line follows per other column, in the record style `Fields` uses.
A blank line separates the rows.
The headers carry the column names, which `Definitions` would drop.

A table with no headers, such as `Table::plain()`, stacks as `Definitions` with the first cell as its label, since it has no names to show.

The alternative was to reuse `stacked_below`'s `Definitions` form.
It lost because a command table's cells mean little without their headers, where a help table's flag and description need none.

### 3. Overrides and disable paths

| Behaviour | Library override | Variable | Disable |
|:--|:--|:--|:--|
| Floor | `RenderOptions::column_floor(cells)`, on `View` and `App` | `CTL_CORE_COLUMN_FLOOR` | `0`: the limits are the overflow limits, as today |
| Stacking | `RenderOptions::table_fit(TableFit)`, on `View` and `App`; `Table::stacking` per table | `CTL_CORE_TABLE_FIT`: `stack`, `wrap` | `TableFit::Wrap` |

`column_floor_envs` and `table_fit_envs` replace each ordered name list, and an empty slice disables lookup, as for the other style options.
An unaccepted value is named in a warning, as today.

### 4. Ellipsis stops at the floor

An ellipsis column is a consumer's choice to cut long text, so it keeps cutting, but at its floor instead of 1 cell.
A table that only fit with 1-cell ellipsis columns now stacks.

### 5. The stacked form is new, not `stacked_below`'s

The option the maintainer chose named `Table::stacked_below` as the fallback.
The fallback keeps that idea, a table that gives way to stacked rows, but draws headed records, for the reason in decision 2.
`stacked_below` itself does not change.

## Risks / Trade-offs

1. [Tables stack on terminals where they wrapped before] → A consumer sets `column_floor` lower or `table_fit(Wrap)`; a user sets `CTL_CORE_TABLE_FIT=wrap`.
2. [Snapshots change in consumers] → The changeset names the rule and both variables.

[widths]: ../../../ctl-layout/src/widths.rs
[render]: ../../../ctl-core/src/render.rs

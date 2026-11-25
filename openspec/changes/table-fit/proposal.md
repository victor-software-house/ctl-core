# Proposal

## Motivation

A table wider than the terminal narrows its widest column one cell at a time, down to the widest word.
A fold or ellipsis column goes down to 1 cell.
A grid table then folds every keep-words column down to its widest grapheme cluster, 1 cell for ASCII text ([widths.rs][widths]).

Take a grid list of six columns of 30-cell paths at width 40.
Its frame takes 19 cells and leaves 21, so by those rules each column folds to 3 or 4 cells.

Rows stack into records today only where a consumer sets a threshold by hand with `Table::stacked_below`.
ctl-core's help does, and command output almost never does.

The maintainer chose the rule for every table: wrap cells down to a floor, then stack each row as a record (operator, 2026-11-24).
A table never crops and never narrows a column to a few characters.

This change belongs to [CTC-064][ledger].

## Changes

1. Each column gets a floor: its header width or 8 cells, whichever is larger, and never more than its natural width.
   No column of a table narrows below its floor, whatever its overflow.
2. A table whose floors, with its frame, do not fit the width draws each row as a stacked record, with no consumer code.
3. **BREAKING** An ellipsis or fold column stops at its floor instead of 1 cell, so a table that relied on that now stacks.
4. The floor is a public option, `RenderOptions::column_floor`, read from `CTL_CORE_COLUMN_FLOOR` when the consumer leaves it open; `0` turns the floor off.
5. Stacking is a public option, `RenderOptions::table_fit`, read from `CTL_CORE_TABLE_FIT` (`stack` or `wrap`), and `Table::stacking` sets it per table.
   `wrap` keeps the floors and lets a table that cannot fit run past the width, as an unframed table does today.
6. A threshold set with `Table::stacked_below` keeps its meaning and its look, so help output does not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `layout`: columns stop at a floor, and a table that cannot fit at its floors stacks.
- `presentation`: the floor and the fit mode are public options with environment variables.

## Impact

1. Code: `ctl-layout/src/widths.rs`, `ctl-layout/src/table.rs`, and a fit choice beside `breakpoint.rs`; `ctl-core/src/render.rs` and `ctl-core/src/document.rs`.
2. Tests: layout snapshots that narrow a column below 8 cells change, to a wrapped cell at the floor or to stacked records.
3. Docs: `docs/presentation.md` and the rustdoc of `Table` in both crates.
4. Release: a `ctl-core: minor` changeset for the changed output, in the 0.8.0 candidates.

[ledger]: ../../../tasks.yaml
[widths]: ../../../ctl-layout/src/widths.rs

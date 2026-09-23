# Proposal

## Why

`choose-visual-identity` made the record style, list style, row separation,
and JSON layout public options, with defaults the operator picked. Only a
library owner can change them, and only by rebuilding the binary. An operator
who wants the boxed record, or compact JSON in one shell, has no way to ask.
The automatic-width buffer already has that path: `CTL_CORE_COLUMN_BUFFER`,
with an ordered, replaceable name list. Task 1.5 of `choose-visual-identity`
asks the same for each style option.

Queue row: CTC-013, "Let the environment override each style option".

## What Changes

1. Read `CTL_CORE_RECORD_STYLE`, `CTL_CORE_LIST_STYLE`,
   `CTL_CORE_ROW_SEPARATION`, and `CTL_CORE_JSON_LAYOUT` when the library
   owner has not set that option.
2. Let `RenderOptions`, `View`, and `App` replace each ordered name list, or
   disable lookup with an empty slice.
3. Ignore a value that is not accepted, and have `App` print one warning line
   per such variable that names the accepted values.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `presentation`: each style option gains an environment override.

## Impact

1. `RenderOptions::record`, `list`, and `separation` read the environment, so
   they are no longer `const`.
2. Output changes only when an operator sets one of the variables.
3. The release is a patch, as every 0.x release is.

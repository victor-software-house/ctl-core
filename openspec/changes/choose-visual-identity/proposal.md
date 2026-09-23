# Proposal

## Why

One table style answers every question. Every `Fields` record and every
`Table` list renders through Comfy Table's `UTF8_FULL_CONDENSED` preset, so a
two-row record gets the same full box as a twelve-column grid. The comparison
in `docs/research/output-rendering-comparison.md` measured ctl-core against
typer with rich and cobra with fang and lipgloss. ctl-core wins on JSON
integrity, width, colour policy, suggestions, and identifier styling. It loses
on four presentation points:

1. A record is boxed. Both alternatives render right-aligned keys with no frame.
2. A list of rows carries a full grid (`┌─┬─┐`, `╞═╪═╡`, `└─┴─┘`). rich's
   `SIMPLE_HEAD` draws one rule under the header and nothing else.
3. Rows run together, and no option separates them.
4. `--format json` prints one unbroken line. A 12-row `qctl status` payload
   cannot be read without `jq`.

Width has two defects as well:

1. The automatic buffer defaults to 1 column. In a host that indents tool
   output, `COLUMNS=119` with buffer 1 renders 118 columns and clips; buffer 2
   renders 117 and is clean. The operator confirmed 2 from a rendered demo.
2. When stdout is not a terminal and `COLUMNS` is unset, `detected_width()`
   returns `None` and nothing caps the table. A schema-4 `qctl show` then
   renders 137 columns. The buffer cannot help, because it is subtracted only
   from a detected width.

The style choices were shown on 2026-09-17 as throwaway rich demos, and the
operator deferred the decision. They were never rendered by ctl-core itself.

Queue row: CTC-010, "Own the unified ctl presentation kernel".

## What Changes

1. Add public style options: a record style, a list style, row separation, and
   JSON layout. Each has a library override and a disable path, as the library
   control rule requires.
2. Add a gallery: one fixed dataset rendered by ctl-core under every candidate
   style, at two widths, coloured and colourless. The operator picks from it.
3. Record the picks in `design.md`, then make them the defaults.
4. Raise the automatic buffer default from 1 to 2.
5. Add a fallback width for the case where no width is detected.
6. Bring the consumer pins forward in verctl, qctl, and forkctl.

## Capabilities

### New Capabilities

- `presentation`: how ctl-core lays out records, lists, JSON, and width.

### Modified Capabilities

None. No spec exists yet.

## Impact

1. Every consumer's pretty output changes on the next pin bump. Snapshot tests
   in all three CLIs change with it.
2. JSON gains a pretty layout. The no-ANSI guarantee does not change.
3. The public API gains style options. Nothing is removed.

# Parity differences

Every way a file in `engine/` differs from its file of the same name in `baseline/`, grouped by kind.
`baseline/` is the comfy-table renderer of 0.6.x and stays frozen;
`engine/` is the ctl-layout renderer.
94 of the 208 files differ.
Each kind names one example as case, width, and mode, and the design decision of the `terminal-render-engine` change that produces it.
No kind is an unfixable regression.

Decision numbers refer to `openspec/changes/terminal-render-engine/design.md`.

## 1. A line that writes a style ends with a reset

Example: `forkctl-no-patches-w80-color`.
`repository  demo-repository` now ends in `\u{1b}[0m`, after the value, although the key's style was already reset.

Reason: decision 5, item 2.
The encoder resets at the end of every line that wrote a style, so each line stands alone in `head`, `less -R`, and a log.
80 of the 84 coloured files that differ show it.

## 2. A run of one style is written once

Example: `qctl-show-queued-w80-color`.
The baseline wrote the title style around every wrapped word, with a reset between words;
the engine writes `\u{1b}[1m\u{1b}[36m` once before the title and resets once.
Help's muted `[env: LEDGERCTL_CONFIG]` in `help-root-w40-color` is the same.

Reason: decision 5, item 1.
The encoder writes a style sequence only where the style changes between spans.
The visible text is the same.

## 3. Colour no longer changes the layout

Example: `help-root-w80-color`.
The baseline's coloured options table is 4 cells wider than its plain one, because comfy-table measured the escape sequences;
the engine's coloured render strips to exactly its plain render.
`qctl-show-archived-w20-color` loses a trailing space only the coloured baseline had.

Reason: decision 5 (layout returns lines of styled spans and never sees an escape byte) and the alternative rejected in decision 9 (comfy-table measures strings with escapes in them).

## 4. An empty column is as wide as its padding

Example: `help-root-w120-plain`.
The help table's empty value column is `┆  ┆`, 2 cells, where the baseline drew `┆   ┆`, 3 cells.
Every help render with an empty argument or value column shows it, and so do the help snapshots in `tests/snapshots/`.

Reason: decision 9, step 1.
A column's width is the widest of its cells plus padding, and an empty cell is 0 cells wide;
comfy-table kept a minimum content width of 1.

## 5. A column that wraps keeps the width it was given

Example: `qctl-status-w80-plain`.
The title column wraps `Document the retry rules…` at the same word as before, but the table now spans all 80 columns, where the baseline shrank the column to its widest wrapped line and drew a 72-column table.
`qctl-check-problems-w40-plain`, `help-root-w80-plain`, and `help-subcommand-w80-plain` show the same;
in `help-subcommand-w80-plain` the wider column also lets `[env:` join the first line.

Reason: decision 9, steps 3 and 5.
The table shrinks a column only until the table fits, then renders each cell at the width the column was given;
it does not narrow the column again after wrapping.

## 6. Columns shrink widest first

Example: `verctl-asset-targets-w40-plain`.
The baseline split `target` mid-word as `macos-arm6` / `4`;
the engine keeps `target` at its natural 11 cells so `macos-arm64` stays whole, and folds the wider `asset` column instead.

Reason: decision 9, step 3.
Wrappable columns shrink widest first toward the next widest, as rich's collapse does, where comfy-table split the width by its own arrangement.

## 7. A word folds only when it alone is wider than its column

Example: `verctl-publish-notes-w40-plain`.
In the 6-cell `note` column, the baseline filled each line cell by cell and split `exists,` after its fourth letter on the line that holds the last letter of `already`;
the engine breaks at spaces first and folds only a word wider than the column, so `exists,` starts its own line as `exists` / `,`.

Reason: the layout requirement "A framed table folds instead of overflowing" and the styled-text requirement "A word is kept whole unless the caller allows a fold":
a framed table folds a word wider than its column, and every other break falls between words.

## 8. Two words never run past the width

Example: `help-root-w20-plain`.
The stacked description line `[default: pretty]`, indented 4, was 21 cells at width 20;
the engine breaks it as `[default:` / `pretty] [env:`.
A single word longer than the width still runs past it, as the indented `LEDGERCTL_FORMAT]` does in both.

Reason: decision 8.
A widget renders lines no wider than the context's width, except a word kept whole.

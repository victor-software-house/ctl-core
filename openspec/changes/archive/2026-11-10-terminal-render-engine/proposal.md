# A terminal rendering engine of ctl-core's own

## Why

ctl-core paints colour into every span before layout.
It hands the painted strings to comfy-table, which parses the escapes again
to measure them.
It then writes through anstream, which parses them a third time to strip
them when the stream is not a terminal.

comfy-table can keep styled cells only with its `tty` feature, which links
crossterm, so every ctl CLI links a terminal library it never drives.

In a toy CLI (opt-level `"s"`, fat LTO, strip, darwin-arm64), an
unpublished prototype engine replaced comfy-table.
It cut 57,052 to 57,436 bytes of `__text` at the `help`, `view` and `app`
tiers.
It also removed 19 packages from the build.

In the four public consumers it cut 28,524 to 45,632 bytes of `__text` and
9 to 14 packages ([measurements][measurements]).
The narrower clap features of [CTC-032][ctc-032] save another 48,000 to
62,000 bytes and are not counted here.

The width rule is also wrong in two places. Help printed to stderr asks stdout
for its width, and the width comes from `/dev/tty` rather than from the
stream being written. And only `Document::verbatim` filters escape sequences
out of text from outside the program, so rawr and other consumers wrote
their own filter.

Beyond replacing comfy-table, the consumers hand-roll what a rendering
library should give them ([consumer needs, D.2][needs-d2]).
The list covers a table with ellipsis and numeric columns, a width rule,
closed-pipe handling, a live progress view, a tree and hyperlinks.

The maintainer asked for an engine at the
level of Python's rich, with ctl-core as typer is to rich.

Queue row: [CTC-034][ledger].

## What Changes

1. ctl-core's repository becomes a Cargo workspace, each crate in a folder at
   the root named like its package. `ctl-core` stays the
   crate a CLI depends on; its features turn on three new leaf crates
   released at the same version:
   - `ctl-text`: styled text, cell widths by grapheme cluster, wrapping and
     truncation, the unsafe-character filter, and the measure-then-render
     protocol;
   - `ctl-layout`: tables, records, rules and the other widgets;
   - `ctl-term`: colour and width detection per stream, the escape encoder,
     and the writer.
2. Escape sequences are written once, by the writer, after layout. comfy-table,
   crossterm, anstream and the ANSI parsers leave every ctl CLI's graph.
3. Text from outside the program has its controls removed in every span, not
   only in `verbatim`.
4. Width is read from the stream being written, then `COLUMNS`, then the
   fallback.
5. `Table` gains `ellipsis_column` and `numeric_column`. A theme maps roles to
   styles and can be replaced. A closed pipe leaves the exit code as the
   command set it: success stays 0, and a failure keeps its code.
6. **BREAKING:** `kv` and `grid`, which no consumer calls, are removed.
   Output captured under `ColorMode::Auto` off a terminal no longer holds SGR
   bytes. Column widths can differ from comfy-table's; the parity test lists
   every difference.
7. `docs/presentation.md`'s rule against writing width, stripping, table
   sizing and detection from scratch is replaced by the rule in
   [design, decision 18][design-18].
8. Later phases, each its own row and change: panels, trees, columns and
   hyperlinks; live regions, progress, spinners and prompts; a Markdown crate.

## Decisions

The maintainer decided these on 2026-11-08 (UTC). Each is argued in
[design.md][design], with the alternatives it beat.

1. The engine ships as three leaf crates in a ctl-core Cargo workspace,
   created now. Input, surface and fixture move to their own leaves in later
   changes ([design, decisions 1 and 2][design]).
2. The crates are named `ctl-text`, `ctl-layout` and `ctl-term`
   ([design, decision 3][design]).
3. A closed pipe exits 0 after success and never hides a failure
   ([design, decision 13][design]).
4. CI enforces a `.text` budget per tier, kept in a
   `[workspace.metadata.size]` table of the root `Cargo.toml`
   ([design, decision 16][design]).

## Capabilities

### New Capabilities

- `styled-text`: how text from outside the program is filtered, how its width
  is counted, and how it wraps, folds and truncates.
- `terminal-output`: when colour is on, at what depth, how wide the output is,
  where escapes are written, and what a closed pipe does.
- `layout`: what every widget guarantees about its width, and how a table
  chooses its column widths.

### Modified Capabilities

- `presentation`: the automatic-width requirement reads the stream being
  written instead of stdout.

## Impact

1. Code: the repository becomes a workspace (`ctl-core/`, `ctl-text/`,
   `ctl-layout/`, `ctl-term/` at the root); `src/render.rs`,
   `src/layout.rs`, `src/style.rs` and `src/table.rs` are replaced; `view.rs`,
   `help.rs` and `app.rs` write through `ctl-term`.
2. Dependencies: added `unicode-segmentation` and `terminal_size`; removed
   `comfy-table`, `anstream`, `unicode-general-category`.
3. Release: one version for the four crates, set by verctl from the
   changesets through its workspace package. The changeset states each break
   in item 6 of What Changes.
4. Consumers: verctl, qctl, forkctl, rawr and the private consumers move their pin
   and review their snapshot diffs. None is expected to change code.
5. Template: the copier answer `layout` moves to `workspace`.

[needs-d2]: ../../../../docs/research/terminal-rendering/consumer-needs.md#d2-hand-rolled-gaps-that-the-library-could-obviously-offer
[design]: design.md
[design-18]: design.md#18-the-rule-against-writing-these-from-scratch-is-replaced
[measurements]: ../../../../docs/research/terminal-rendering/measurements.md
[ledger]: ../../../../tasks.yaml
[ctc-032]: https://github.com/victor-software-house/ctl-core/pull/183

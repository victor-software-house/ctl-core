# Design

## Goals

1. Each kind of content gets the lightest frame that still reads: a record, a
   list, a grid, and a JSON payload each have their own layout.
2. The operator chooses the defaults from real ctl-core output, not from
   another framework's rendering.
3. Output never exceeds the usable width, detected or not.

## Non-goals

1. A `completion` verb for qctl and verctl, and examples in command help.
   Those belong to each CLI.
2. Colour themes beyond the existing terminal theme. The brand palette in
   `docs/brand.md` applies to marks and banners, not to terminal output.
3. Coloured JSON. It stays opt-in and out of scope here.

## Decisions

### 1. Styles are options with defaults, not fixed presets

Each axis is a public enum on `RenderOptions`, reachable through `View` and
`App`, with an environment override named like `CTL_CORE_COLUMN_BUFFER`.

Alternative: pick one new look and hard-code it. It lost because the library
control rule requires an override for every automatic behaviour, and because
a consumer such as a patch-stack view may need a full grid where a queue does
not.

### 2. The gallery renders through ctl-core

`examples/gallery.rs` renders one fixed dataset: a queue list, a single record,
a wide grid, an error, and a JSON payload. It renders every candidate at width
80 and 120, with `--color always` and `--color never`. It runs on the build
host. Its ANSI output is converted to one HTML page for review.

Alternative: rebuild the typer and cobra demos. That lost because they showed
another renderer's look, which is the thing being replaced.

### 3. Candidates

| # | Axis | Candidates | Today |
|--:|:--|:--|:--|
| 1 | Record | `Boxed`, `Borderless` with right-aligned keys, `Borderless` with left-aligned keys | `Boxed` |
| 2 | List | `Grid`, `HeaderRule`, `Plain` with no rules | `Grid` |
| 3 | Row separation | `None`, `Rule` between rows, `Blank` line between rows | `None` |
| 4 | JSON | `Compact`, `Pretty` with two-space indent, `Pretty` only when stdout is a terminal | `Compact` |

The recommendation from the comparison is: `Borderless` right-aligned, then
`HeaderRule`, then `None` for separation with `Rule` opt-in, then `Pretty`
when stdout is a terminal. The operator decides.

### 4. Width

1. `DEFAULT_COLUMN_BUFFER` becomes 2. Operator decision, 2026-09-17.
2. When no width is detected, ctl-core renders at a floor of 80 columns.
   The detection order stays stdout, then `COLUMNS`. A consumer can change the
   floor or disable it through `RenderOptions`.

Alternative for 2: probe stderr, stdin, and `/dev/tty` before the floor. It
lost because agent and CI captures run with every descriptor piped and no
controlling terminal, so the floor is the rung that fires in practice. Walking
the process tree to find a terminal is out of bounds for a library.

## Operator decisions

Record each pick here with the date, after the gallery review.

| # | Axis | Pick | Date |
|--:|:--|:--|:--|
| 1 | Record | pending | |
| 2 | List | pending | |
| 3 | Row separation | pending | |
| 4 | JSON | pending | |
| 5 | Buffer default | 2 | 2026-09-17 |
| 6 | Width floor | pending | |

## Risks

1. Snapshot churn in three consumers. Each pin bump is its own pull request.
2. A borderless record loses visual grouping when two records print in a row.
   The gallery shows two consecutive records to test this.

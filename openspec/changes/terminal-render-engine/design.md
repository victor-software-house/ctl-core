# Design

## Context

ctl-core draws every human view of the ctl CLIs: records, lists, notices,
help. It does so in three passes over the same bytes
([consumer needs, A.5][needs-a5]):

1. `Renderer::paint` writes SGR escapes into each span before layout, for
   `--color auto` as well as `always` ([render.rs:662][render-paint]).
2. comfy-table measures and wraps those strings, so its `custom_styling`
   feature parses the escapes again through `ansi-str` and `console`. Plain
   paragraphs go through a one-cell comfy table too
   ([render.rs:609][render-wrap]), and the terminal width comes from
   `comfy_table::Table::width()`, which asks crossterm about `/dev/tty`
   ([layout.rs:16][layout-detect]).
3. `anstream::AutoStream` parses the escapes a third time to strip them when
   the stream is not a terminal.

`custom_styling` turns on comfy-table's `tty` feature, which links crossterm.
The research behind this design is in
[docs/research/terminal-rendering][research]: Python's rich and typer, Charm's
lipgloss and glamour, 13 Rust layout crates and 7 Rust ports of rich, 19 Rust
styling and terminal crates, every consumer's call sites, and binary-size
measurements of each candidate.

The approved crate-structure rules apply: the `rust-projects` skill, section 4
items 19 and 20 and section 5 items 4, 9 and 11. In short: a crate starts
where a heavy dependency or an independent consumer starts; the facade keeps
the family's bare name; leaves are `ctl-<noun>`; every crate in the
workspace releases at one version; no generic code crosses a crate boundary.

## Goals / Non-Goals

**Goals:**

1. One rendering engine, written for the ctl CLIs and usable alone, that
   replaces comfy-table and anstream for every purpose ctl-core has today.
2. Escape sequences written once, by the sink, after layout. Layout never sees
   an escape byte, so nothing strips, re-parses or repairs one.
3. The same `Document` API for consumers, so a consumer adopts by moving its
   ctl-core pin and reviewing its snapshot diffs.
4. A measured size win at every feature tier and in every consumer, and no
   build-time loss.
5. The widgets rich has that the consumers already hand-roll: per-column width
   policy with ellipsis and numeric alignment, sanitized spans, tree, panel,
   hyperlinks, live progress. The last three come in later phases (decision
   14).

**Non-Goals:**

1. A full-screen terminal UI. rawr's picker uses ratatui and keeps it. The engine draws lines into a stream; it owns no
   alternate screen, raw mode or input events.
2. Syntax highlighting and diff rendering. They serve a full-screen client,
   not a CLI chassis, and each would bring a heavy dependency.
3. Markup parsing (`[bold]text[/]`) and automatic highlighting. rich's own
   history shows both corrupt data ([rich survey, 9.2][rich-flaws], flaws 1
   and 2).
4. Parsing ANSI that a child process wrote. Relayed child output stays a byte
   pass-through, and untrusted text has its controls removed (decision 7).

## Decisions

### 1. The engine ships as three leaf crates in the ctl-core repository

ctl-core's repository becomes a Cargo workspace. The engine is three leaf
crates beside the `ctl-core` facade: `ctl-text`, `ctl-layout` and `ctl-term`
(decision 3). All four release together at one version.

Alternatives:

1. **A private module in ctl-core**, as the unpublished prototype did. It
   lost because consumers want a piece of the engine without the rest
   ([consumer needs, D.2][needs-d2]): rawr wrote its own escape-sequence
   filter and wrapper, and private consumers, checked but not named here,
   wrote their own styled text, width rule and table. A module inside ctl-core gives them clap, serde and the CLI chassis as well.
2. **A Cargo feature of ctl-core.** Same reach problem, and features unify
   across a build and spread `#[cfg]`. The approved rules make a crate the
   default way to make a piece opt-in.
3. **Its own repository**, released separately. It lost on lockstep. Cargo
   treats every `0.0.x` as incompatible with every other, so each engine
   release would need a ctl-core release before any consumer could move, and
   a change that touches the facade and the engine would be two pull requests
   in two repositories. Today every consumer reaches the engine through
   ctl-core. The crate boundaries in decision 3 are what a later move would
   need, so the repository can split without renames once the engine has a
   consumer that does not use ctl-core. verctl already releases a virtual
   workspace as one package and publishes its members in dependency order
   ([verctl members.rs][verctl-members]), so the release lane stays one
   Version PR.
4. **Adopt an existing crate.** comfy-table has no configuration that keeps
   styled cells without crossterm ([layout survey, 3][layout-comfy]). tabled
   rewrites cells to wrap them and parses ANSI behind a feature that changes
   its width answer. The rich ports are young: the most used has 6 releases,
   and the most literal ships its own width tables and has 174 open issues
   ([layout survey, 10][layout-ports]). None separates measuring from
   rendering for arbitrary nested content except the ports. Measured costs
   are in decision 16.

### 2. ctl-core becomes a virtual workspace now, with each crate in a root folder, and its other modules move later, one change each

The repository root holds a virtual `Cargo.toml` with `resolver = "3"`,
`[workspace.package]` version, `[workspace.lints]` and
`[workspace.dependencies]` with `default-features = false`. Every crate lives
in a folder at the repository root named like its package (`ctl-core/`,
`ctl-text/`, `ctl-layout/`, `ctl-term/`), as the `rust-projects` skill names
crate folders, and sets `[lints] workspace = true`. The copier answer
`layout` moves from `crate` to `workspace`, which the shared template already
supports, and the baseline renders the workspace form.

What moves where:

| Today | This change | Later, with evidence |
|:--|:--|:--|
| `src/render.rs` engine half, `src/layout.rs`, `src/style.rs`, `src/table.rs` | `ctl-layout`, `ctl-text`, `ctl-term`; `table.rs` (`kv`, `grid`) is deleted, no caller exists | |
| `src/verbatim.rs` (from CTC-032) | `ctl-text` | |
| `src/document.rs`, `src/render.rs` mapping half, `src/view.rs`, `src/help.rs`, `src/app.rs`, `src/flags/`, `src/parser.rs`, `src/color.rs`, `src/format.rs`, `src/model.rs` | stay in `ctl-core` | |
| `src/input.rs`, `src/input/`, `src/validate.rs` | stay | `ctl-input`: serde-saphyr, yamled, garde and toml are heavy dependencies a CLI may want without the chassis |
| `src/served.rs`, `src/surface.rs`, `src/surface/`, `src/usage.rs` | stay | `ctl-surface`: MiniJinja and usage-lib |
| `src/fixture.rs` | stay | `ctl-fixture`: a dev-only consumer with its own dependency, tempfile |

Alternatives:

1. **A `crates/` folder** holding every member, as BeautifulRust's
   M-CRATES-FLAT-FOLDER suggests. It lost: the house rule names a crate's
   folder like its package at the root, and no measurement or source shows a
   gain from the extra level.
2. **Move everything now.** It lost on scope: each later move has its own
   consumers to migrate and its own evidence to collect, and folding them in
   would make this change unreviewable.
3. **Keep a single crate and put the engine in a second repository.** It
   lost under decision 1.

### 3. Three leaves, split where a dependency or an independent consumer starts

| Crate | Holds | Depends on | Independent consumer |
|:--|:--|:--|:--|
| `ctl-text` | styled text (`Span`, `Line`, `Text`), the cell-width model, wrapping and truncation, the unsafe-character set, and the renderable protocol (`Renderable`, `Measurement`, `Context`) | `anstyle`, `unicode-width`, `unicode-segmentation`, `unicode-bidi` | rawr's title filter and wrapper; private consumers, checked but not named here |
| `ctl-layout` | widgets built on the protocol: `Table`, `Record`, `Rule`, `Stack`, `Indent`, later `Panel`, `Tree`, `Columns`, `ProgressBar` | `ctl-text` | private consumers, checked but not named here |
| `ctl-term` | terminal facts (`Terminal`: is a terminal, colour depth, width), colour policy, the escape encoder, the `Console` writer, later `Live` and prompts | `ctl-text`, `anstyle`, `terminal_size` | rawr's terminal check; private consumers, checked but not named here |
| `ctl-core` | the facade and the CLI chassis: `App`, flags, `Document`, the `Theme` that maps roles to styles, `View`, help, JSON | the three leaves behind features | every CLI |

The maintainer chose these names on 2026-11-08 (UTC). All three were free on
crates.io. The names considered and not taken:

| Crate | Alternatives considered |
|:--|:--|
| `ctl-text` | `ctl-span`, `ctl-styled` |
| `ctl-layout` | `ctl-widgets`, `ctl-render`, `ctl-blocks` |
| `ctl-term` | `ctl-tty`, `ctl-console` |

`ctl-layout` and `ctl-term` do not depend on each other. `ctl-term` writes
lines of styled spans, which is plain data from `ctl-text`; it never needs a
widget. A widget never needs a terminal. That keeps the graph wide and
shallow: `ctl-layout` and `ctl-term` compile in parallel once `ctl-text` is
built, and the layout crates link no `rustix` or `libc`.

The protocol sits in `ctl-text` because both other leaves need it: `ctl-term`
will redraw any `&dyn Renderable` in a live region, and `ctl-layout`
implements it. A fourth interface crate holding one trait was considered and
lost: it has no dependency or consumer of its own, so the rules keep it a
module.

How the two rules are reconciled. BeautifulRust prefers small single-purpose
crates in a flat workspace (Microsoft M-SMALLER-CRATES and
M-CRATES-FLAT-FOLDER). The `rust-projects` skill splits only at a boundary
shown by a dependency, a second consumer or a measurement. Where they
disagree, the skill wins, because a split with no evidence is the boundary
Wasmtime merged back. Workspace-wide versions and lints, and individual
re-exports, come from BeautifulRust and apply in full; its `crates/` folder
does not (decision 2).
So `Table`, `Tree` and `Panel` share `ctl-layout`: they share every
dependency and no consumer wants one without the others. A widget that brings
a heavy dependency becomes its own crate: `ctl-markdown` with
`pulldown-cmark`, in a later phase.

Alternatives:

1. **One engine crate.** It lost because `ctl-term` brings `rustix` (and
   `windows-sys` on Windows) and the text consumers want neither, and because
   it would make the pure layout code depend on the operating system.
2. **A crate per widget** (`ctl-table`, `ctl-tree`, `ctl-panel`). It lost
   under the skill: no dependency or consumer separates them, and LTO already
   drops an unused widget, so the split would shrink nothing.
3. **A separate `ctl-style` crate.** It lost because the style type is
   `anstyle::Style` (decision 4), so there is nothing left to put in it.

### 4. The style type is `anstyle::Style`

`ctl-text` re-exports the `anstyle` crate and uses `anstyle::Style` for every
span. A hyperlink is not a style attribute; a `Span` carries an optional link.
`ctl-text` adds the one operation anstyle lacks, `overlay(base, top)`, where
the top style's colours and effects win.

Alternatives:

1. **Our own style type**, with rich's tri-state attributes. It lost on
   reuse: anstyle is `Copy`, `const`, has no dependency, is stable at 1.0,
   and is what clap's `Styles` and cargo already use, so a CLI shares one type
   with its parser. clap_builder links anstyle unconditionally, so it costs a
   ctl CLI nothing. Tri-state inheritance is only needed for markup, which is
   a non-goal.
2. **owo-colors or yansi.** They write escapes at format time and decide
   colour from global state, which is the opposite of decision 5.

### 5. Escape sequences are written once, by the sink

Layout returns `Vec<Line>`, each line a list of `Span { text, style, link }`.
No escape byte exists before `ctl-term`'s encoder. The encoder:

1. writes an SGR sequence only where the style changes between spans, at the
   colour depth the terminal supports (decision 11);
2. resets at the end of every line, so each line stands alone in `head`,
   `less -R` and a log;
3. opens and closes OSC 8 around a linked span when links are on;
4. writes plain text, byte for byte, when colour is off.

So `anstream`, `anstyle-parse`, `utf8parse`, `colorchoice`, `ansi-str`,
`ansitok`, `nom`, `vte` and `console` leave the graph. On Windows, `ctl-term`
turns on the console's virtual-terminal mode through `anstyle-query`'s
`windows::enable_ansi_colors` and treats a failure as no colour.

Alternatives:

1. **Paint first, strip later**, as today and as lipgloss does on every
   operation ([Charm survey, 7.2][charm-weak]). It lost: it costs a parser,
   it makes captured output depend on the strip, and layout has to measure
   around escapes it did not need to see.
2. **Keep anstream as the writer.** `AutoStream::new(writer, Always)` reads
   no environment, so it could sit under the encoder. It lost because with
   the decision made before rendering it would only add a parser that never
   finds anything to strip. Measured cost: anstream adds 5,588 bytes of
   `__text` and 6 packages over anstyle alone ([measurements, A3][m-a3]).

### 6. Width is counted in terminal cells per grapheme cluster

1. A grapheme cluster comes from `unicode-segmentation`; its width is
   `unicode-width`'s `str` width of the cluster, which already applies emoji
   ZWJ, modifier and presentation sequences. A ZWJ family is 2 cells, not 6.
2. East Asian ambiguous characters count 1 cell. `unicode-width`'s `cjk`
   feature stays off, which also keeps its second table out of the binary.
3. A tab expands to the next multiple of 8 cells from the start of its line
   when text is built, so layout never measures a tab.
4. Storage offsets are bytes into `str`. Widths are cells. The two units
   never share a type: a width is `u16`, an offset is `usize`.
5. A cut that falls inside a 2-cell cluster replaces the cluster with a space
   on each side, as rich does, so every line keeps its exact width.
6. `ctl_text::UNICODE_VERSION` states both crates' Unicode versions, and a
   test pins them, so an upgrade that moves one forces a recheck (the
   complete-sets rule). Today segmentation is Unicode 17 and width is 18
   ([layout survey, 7.2][layout-width]).

Alternatives:

1. **Sum of per-`char` widths** (what comfy-table and the prototype do). It
   lost: it miscounts every emoji sequence and splits clusters on a fold.
2. **rich's special cases** (extend over ZWJ and VS16 only). It lost under the
   complete-sets rule: regional indicators, Hangul jamo and Indic conjuncts
   are left to chance.
3. **No segmentation, break only at whitespace.** It would save the
   segmentation tables (5,248 bytes of `__text`, [measurements, A4][m-a4]),
   but a table that must fold a long
   word would still need cluster boundaries.

### 7. Text from outside the program is filtered when a span is built

`Span::new` drops every character `verbatim::is_unsafe` refuses (the C0 and
C1 controls including ESC, the twelve UAX #9 directional formatting
characters, and the line and paragraph separators), splits on newline, and
expands tabs. So a commit subject or a session title cannot move the cursor,
reorder the line or break the layout, in any widget, without each consumer
filtering by hand. The set and its sources are CTC-032's module, which moves
into `ctl-text` unchanged. `Document::verbatim` keeps its rule (no wrapping,
newlines and tabs kept).

Alternative: **filter only in `verbatim`**, as today. It lost: rawr, the agent
runtime and its terminal client each wrote their own filter
([consumer needs, D.2][needs-d2], gap 5), and rich's equivalent strips only 5
characters and lets ESC through ([rich survey, 9.2][rich-flaws], flaw 3).

### 8. Every widget measures, then renders

```rust
pub trait Renderable {
    /// The narrowest and widest this content can usefully be, in cells.
    fn measure(&self, context: &Context) -> Measurement;
    /// Lines no wider than `context.width()`, except a word kept whole.
    fn render(&self, context: &Context) -> Vec<Line>;
}
```

1. `Measurement { minimum, maximum }`: the minimum is the widest unit the
   content will not break (a word under `Overflow::KeepWords`, a cluster
   under `Overflow::Fold`), the maximum its natural width.
2. `Context` carries the target width, the tab width, and the box character
   set (Unicode or ASCII). It holds no colour or terminal fact: styles are
   resolved before layout and escapes after it.
3. A container measures its children, decides their widths, and renders each
   child at its width. Its result is a rectangle of lines, so borders,
   padding and side-by-side columns are line concatenation.
4. The trait is object-safe. Containers hold `Box<dyn Renderable>`, so no
   generic code crosses from `ctl-layout` into its callers.

Alternatives: **ratatui's constraint solver.** It lost: it solves user
constraints for nested UI regions and never measures content
([layout survey, 11][layout-measure]). **papergrid's estimate-then-draw.** It
covers tables only. rich's split covers any nesting and is what the ports
copy.

### 9. The table chooses column widths in five steps

For a table laid out to width `W`, with frame and gaps taking `F` cells:

1. Measure each column: the maximum and minimum over its cells, plus padding.
   A column with `Overflow::Ellipsis` or `Overflow::Fold` has minimum 1.
2. If the maxima fit in `W - F`, use them.
3. Otherwise shrink the wrappable columns, widest first, toward the next
   widest, as rich's collapse does, but never below a column's minimum.
4. If the minima still do not fit, shrink `Ellipsis` and `Fold` columns to
   their floor of 1, then let `KeepWords` columns run past `W`. A framed table
   folds instead, to keep its borders whole, as today.
5. Render each cell at its width. An `Ellipsis` cell ends in `…`; a numeric
   column aligns right.

This fixes two flaws found in rich ([rich survey, 9.2][rich-flaws], flaw 10):
its collapse ignores minimum widths, and its last step can shrink fixed and
no-wrap columns. Step 3 keeps the prototype's useful rule that a column whose
content fits its share takes its content width.

Alternative: **port comfy-table's dynamic arrangement.** It lost because it
measures strings with escapes in them, and the result cannot be described in
a spec: parity snapshots (decision 17) show where the new algorithm differs.

### 10. Terminal facts are a pure function of an environment snapshot

`Terminal::detect(stream, mode, env)` returns `{ is_terminal, color, width,
links }`. `env` is a lookup function, so a test passes a map and no test
touches process state. `Terminal::from_process(stream, mode)` passes
`std::env::var`.

Colour, for `ColorMode::Auto`, first match wins:

1. `NO_COLOR` set and non-empty: no colour.
2. `CLICOLOR_FORCE` set, non-empty and not `0`: colour.
3. `FORCE_COLOR` set, non-empty and not `0` or `false`: colour.
4. `CLICOLOR=0`: no colour.
5. `TERM=dumb`: no colour.
6. The stream is a terminal: colour. Otherwise none.

`ColorMode::Always` and `Never` read no variable. The decision is made once,
before rendering, for the stream the output goes to.

Width, first match wins: an explicit width (exact); the window size of the
stream being written, through `terminal_size::terminal_size_of`; `COLUMNS`;
the fallback (80). The buffer and the minimum apply to a detected width as
today. Help printed to stderr now asks stderr, not stdout
([consumer needs, A.6][needs-a6]).

Alternatives:

1. **anstream's detection.** It does not read `FORCE_COLOR`, re-detects on
   every print, and is tied to its writer ([style survey, 3.1][style-auto]).
2. **crossterm's size.** It asks `/dev/tty`, not the stream, so it is wrong
   for redirected output, and it links `parking_lot` and `mio`.
3. **rich's order.** It reads the width from stdin first and treats
   `FORCE_COLOR=0` as force ([rich survey, 9.2][rich-flaws], flaws 12 and
   13).

### 11. Colour depth is detected and every colour is downgraded to it

Depth: `COLORTERM` of `truecolor` or `24bit`, or `FORCE_COLOR=3`, is 24-bit;
`TERM` containing `256color`, or `FORCE_COLOR=2`, is 256; anything else with
colour on is 16. The encoder maps a 24-bit colour to the nearest of the 256
palette and a 256 colour to the nearest of the 16, as a pure function. No
surveyed crate does this ([style survey, 5.3][style-depth]). The default theme
uses only the 16 named colours, so its output is the same at every depth.

### 12. Roles map to styles through a public theme

`ctl_core::Theme` maps each `Role` to an `anstyle::Style`. `Theme::default()`
is today's palette ([style.rs][style-rs]). `App::theme`, `View::theme` and
`RenderOptions::theme` replace it; `ColorMode::Never` disables it. The theme
is applied when `Document` becomes widgets, so `ctl-layout` never sees a role.

### 13. A closed pipe ends the program quietly

The maintainer chose exit code 0 on 2026-11-08 (UTC). When stdout is closed
by the reader (`| head`), `Console` reports `BrokenPipe`, and `App` exits with
code 0 and writes nothing to stderr. Today `App` exits with a failure, so a
consumer must catch the error by hand. The alternative, exit 141 as a shell does on SIGPIPE, was considered; it lost
because a ctl CLI is not killed by the signal, and a non-zero exit fails
`set -o pipefail` scripts for a reader that simply had enough. A consumer that
wants the error sets `App::broken_pipe(BrokenPipe::Fail)`.

### 14. Phases: replace first, then add what rich has

| Phase | Delivers | Row |
|:--|:--|:--|
| 1 (this change) | the workspace, the three leaves, `Table` with per-column overflow and numeric alignment, `Record`, `Rule`, `Stack`, `Indent`, paragraphs and headings, the encoder, detection, `Console`, the theme, closed-pipe handling; ctl-core on the new engine; comfy-table and anstream removed | CTC-034 |
| 2 | `Panel`, `Tree`, `Columns`, `Align`, hyperlinks (OSC 8 with an allow-list and `FORCE_HYPERLINK`), a `Table` footer and title | new row |
| 3 | `Live` region, `ProgressBar`, `Spinner`, `Status`, confirmation and secret prompts | new row |
| 4 | `ctl-markdown`: the bundled instructions rendered instead of printed raw | new row |

Left out on purpose: markup, highlighters, syntax highlighting, diff, export
to HTML or SVG, pretty-printing of arbitrary data, tracebacks. Each either
corrupts data, needs a heavy dependency, or serves no consumer
([consumer needs, C][needs-c]).

### 15. Public API

A consumer's code does not change:

```rust
use ctl_core::prelude::*;

impl Present for Status {
    fn present(&self) -> Document {
        Document::new()
            .fields(Fields::new().row("ledger", "tasks.yaml").row("active", Text::new().id(&self.active)))
            .table(
                Table::new(["id", "title", "age"])
                    .id_column(0)
                    .ellipsis_column(1)
                    .numeric_column(2)
                    .stacked_below(80, 2)
                    .rows(self.rows.iter().map(Row::cells)),
            )
    }
}
```

`ellipsis_column` and `numeric_column` are the two new semantic options.
They say what a column holds, not how it is drawn.

A CLI that needs a widget `Document` does not have uses the engine through
the facade's re-exports (`ctl_core::layout`, `ctl_core::term`,
`ctl_core::text`) or the leaf crates directly:

```rust
use ctl_core::layout::{Column, Overflow, Table};
use ctl_core::term::{Console, Stream};
use ctl_core::text::{Text, anstyle::AnsiColor};

let console = Console::from_process(Stream::Stdout, ColorMode::Auto);
let table = Table::new([
    Column::new("title").overflow(Overflow::Ellipsis),
    Column::new("age").numeric(),
])
.row([Text::plain(&title), Text::plain(age.to_string())]);
console.print(&table)?;
```

A new widget implements the protocol:

```rust
use ctl_core::text::{Context, Line, Measurement, Renderable, Span, Style};

struct Meter { done: u16, total: u16 }

impl Renderable for Meter {
    fn measure(&self, _: &Context) -> Measurement {
        Measurement::exact(12)
    }

    fn render(&self, context: &Context) -> Vec<Line> {
        let filled = context.width().min(12) * self.done / self.total.max(1);
        vec![Line::from(Span::new("█".repeat(filled.into()), Style::new().bold()))]
    }
}
```

Rules every public item follows: no `bool` parameter (an enum instead); a
builder method converts its argument at once and calls a concrete inner
function, so its generic shim stays small; `#[non_exhaustive]` on public
enums; `Debug` on every public type; `# Errors` on every fallible function.

### 16. Feature tiers and their measured sizes

ctl-core's features turn on leaves:

| ctl-core feature | Leaves | New compared with today |
|:--|:--|:--|
| `document` | none | unchanged |
| `render` | `ctl-text`, `ctl-layout` | renders to a `String`; no terminal crate, no `rustix` |
| `help`, `view` | add `ctl-term` | detection and the writer |
| `app` | as today | unchanged |

Measured on a toy CLI at each tier, ctl-core 0.6.28 against the unpublished
prototype engine, with opt-level `"s"`, fat LTO, one codegen unit, abort and
strip on darwin-arm64 ([measurements, Matrix B][m-b]). The prototype also
narrows clap's features, so a third build isolates that change; the engine
column is what this change owns:

| Tier | `__text`, 0.6.28 | `__text`, prototype | of which clap features | of which engine | Built packages |
|:--|--:|--:|--:|--:|--:|
| `cli` | 361,288 | 299,272 | -62,016 | 0 | 22 → 15 |
| `help` | 476,592 | 370,596 | -48,560 | -57,436 | 46 → 27 |
| `view` | 462,376 | 345,112 | -60,212 | -57,052 | 52 → 34 |
| `app` | 499,076 | 393,452 | -48,244 | -57,380 | 53 → 35 |

The leaves' own dependencies cost little over a plain `println!` baseline:
anstyle 1,620 bytes, unicode-width with unicode-segmentation 10,232,
terminal_size 788 and anstyle-query 596. comfy-table as ctl-core sets it
today costs 80,288, crossterm's `terminal::size` alone 23,396
([measurements, Matrix A][m-a]). `cargo bloat` on the 0.6.28 `app` tier puts
comfy-table and the crates it brings at about 55 KiB, which agrees with the
engine column.

Per consumer, the prototype cut `__text` by 44,096 (verctl), 45,632 (qctl),
44,544 (forkctl) and 28,524 (rawr) with each one's own release profile
([measurements, Matrix C][m-c]). rawr keeps crossterm and
unicode-segmentation through ratatui. The private consumers were not
measured. Clean release builds used 16% to 35% less CPU time; wall time
moved by less than the noise of the shared host.

A budget task, `check:size`, builds the toy at each tier in CI on linux-x64
and fails when `.text` grows more than 2% over that tier's budget. The
budgets are build configuration for this repository's own crates, so they
live where the repository already keeps tool configuration for its build:
Cargo metadata, as `[package.metadata.docs.rs]` and
`[package.metadata.cargo-machete]` do. They form a `[workspace.metadata.size]`
table in the root `Cargo.toml`. The task reads it with
`cargo metadata --format-version 1 --no-deps --locked | jq`, using the jq the
repository already pins, and measures `.text` with binutils' `size -A`, which
the Linux runner already has.

Where the budgets live, alternatives:

1. **A `size:` section in `.ctl/`**, such as the retired `.ctl/rust.yaml`
   ([PR 101][pr-101]). It lost: `.ctl/` holds configuration a ctl CLI reads,
   such as `.ctl/ver.yaml` for verctl, and no ctl tool reads size budgets.
2. **A new file of their own**, such as `.ctl/size.yaml`. It lost for the
   same reason, and because Cargo metadata already holds this repository's
   build-tool configuration.

Existing tools that enforce a binary-size budget were checked first
(2026-11-08, UTC). None fits without adding a service, a runtime, or an
unmaintained dependency:

| Tool | What it does | Why it is not used |
|:--|:--|:--|
| [Bencher][bencher-size] | tracks file size per run with thresholds | needs a Bencher server or account, a new service; measures the whole file, not `.text` |
| [sizewatcher][sizewatcher] | compares sizes between a pull request and its base, with percentage limits | a Node tool, which the repository would add only to run it; compares branches, not a budget |
| [cargo-size][cargo-size] 0.1.1 | reports and compares binary sizes | leaves the budget check to the caller; one release, one author |
| [cargo-bloat][cargo-bloat] 0.12.1 | attributes `.text` to crates and functions | an analysis tool with no budget; used for the research |
| [cargo-bloat-action][cargo-bloat-action] | posted size diffs on pull requests | archived since 2022 |

Other projects that gate size write the same small task themselves, against
budgets committed in the repository.

Alternative: measure by hand at each release, as the narrow-clap change did.
It lost because a size regression would ship before anyone measured.

### 17. Tests

1. **Snapshots.** Every widget and every `Document` block has insta file
   snapshots at widths 20, 40, 80 and 120, colourless and with colour, under
   `<crate>/tests/snapshots/`.
2. **Parity.** A corpus of the documents consumers render (taken from their
   snapshot and capture tests) is rendered by ctl-core at the last
   comfy-table release and by the new engine. The parity test commits both
   outputs; each difference is listed in the PR with its reason, and none is
   accepted silently.
3. **Properties** (proptest), over random text with wide, combining, emoji and
   control characters, and random widths from 1 to 200:
   - no rendered line is wider than the width, except a line holding a word
     kept whole, and then only by that word;
   - every non-space character of the input appears once, in order;
   - no grapheme cluster is split;
   - a rendered width never falls below `measure().minimum` unless the target
     width is below it;
   - colourless output contains no ESC byte; coloured output resets at the
     end of every line.
4. **Detection.** A table test of `Terminal::detect` over every variable in
   decision 10, each alone and in conflicting pairs.
5. **Consumers.** Each CLI keeps its test that runs the binary with
   `CLICOLOR_FORCE=1` and finds no ESC byte in JSON.

### 18. The rule against writing these from scratch is replaced

[presentation.md][presentation] says: "Do not implement Unicode width, ANSI
stripping, table sizing, or terminal capability detection from scratch." The
new rule keeps two parts and replaces two:

1. Kept: Unicode width and segmentation come from `unicode-width` and
   `unicode-segmentation`; the window size from `terminal_size`; the
   terminal check from `std::io::IsTerminal`.
2. Replaced: ANSI stripping is not written at all, because nothing is painted
   before the decision to paint (decision 5).
3. Replaced: table sizing is written in `ctl-layout`, because no crate sizes
   styled cells without a terminal crate and a second escape parser, and the
   one that comes closest still ignores minimum widths. The algorithm is
   specified (decision 9) and property-tested (decision 17).
4. Replaced: the colour policy is written in `ctl-term` as a pure function,
   because no crate reads both `NO_COLOR` and `FORCE_COLOR` with an explicit
   override, per stream, without global state.

## Automatic behaviours

| Behaviour | Public override | Disable path |
|:--|:--|:--|
| Colour decided from the environment and the stream | `ColorMode::Always`, `--color always` | `ColorMode::Never`, `--color never`, `NO_COLOR` |
| Colour depth from `COLORTERM`, `TERM`, `FORCE_COLOR` | `Terminal::with_depth(depth)` | `ColorMode::Never` |
| Width from the stream, then `COLUMNS` | `width(n)` on `RenderOptions`, `View`, `App` | an explicit width |
| Buffer and minimum on a detected width | as today | as today |
| Fallback width 80 | `fallback_width(Some(n))` | `fallback_width(None)` |
| Unsafe characters removed from spans | none: it is a safety rule | `Document::verbatim` keeps tabs and newlines |
| Tabs expanded to 8 | `Context::tab_width(n)` | none |
| Closed pipe exits 0 | `App::broken_pipe(BrokenPipe::Fail)` | the same |
| Default theme | `App::theme`, `View::theme`, `RenderOptions::theme` | `ColorMode::Never` |
| Windows virtual-terminal mode | none | `ColorMode::Never` |

## Risks / Trade-offs

1. [Output differs from comfy-table in column widths] → the parity test
   (decision 17) lists every difference, and each consumer reviews its
   snapshot diff in the pull request that moves its pin.
2. [We now own a table algorithm] → it is specified in five steps and checked
   by properties, and its cases are a subset of rich's, whose behaviour is on
   record.
3. [`ctl_core::Text` (roles) and `ctl_text::Text` (styles) share a name] →
   the facade re-exports the leaf as a module (`ctl_core::text::Text`), never
   at the root, and the docs say which is which.
4. [`unicode-segmentation` and `unicode-width` disagree on a new emoji] → the
   pinned version test fails on any upgrade that moves one of them.
5. [Four crates to release] → verctl publishes a virtual workspace as one
   package at one version, in dependency order.
6. [Captured output changes for `ColorMode::Auto`] → today a capture under
   `Auto` holds SGR bytes because the strip happens at write time. After this
   change, `Auto` off a terminal paints nothing. Consumer tests use `Never`
   ([consumer needs, A.5][needs-a5]), so none should break; the changeset
   says so.

## Migration Plan

1. ctl-core: workspace conversion, the three leaves, the facade on top, in
   one release. Removing `kv` and `grid` (no caller) and dropping SGR from
   output captured under `ColorMode::Auto` off a terminal are breaks; the
   changeset states both. verctl sets the version from the changesets.
2. Each consumer moves its pin in its own pull request and reviews its
   snapshot diff:

| Consumer | Uses | Code change | Gains |
|:--|:--|:--|:--|
| verctl | `Fields`, `Table`, `Notice`, `verbatim`, `Present`, `App`, `View`, `capture` | none expected | -44,096 B `__text`, -11 packages |
| qctl | as verctl, plus `Section`, role spans, `stacked_below(80, 2)` | none expected | -45,632 B `__text`, -10 packages |
| forkctl | as verctl, plus `Section` | none expected | -44,544 B `__text`, -14 packages |
| rawr | `Present`, `App`, `verbatim` | none expected; may replace its own title filter with `ctl_core::text` | -28,524 B `__text`, -9 packages; less because ratatui keeps crossterm |

Private consumers move their pins the same way; they are not listed here.

Rollback: a consumer pins the previous ctl-core release. ctl-core keeps no
compatibility path for comfy-table.

[needs-a5]: ../../../docs/research/terminal-rendering/consumer-needs.md#a5-pipeline-from-document-to-bytes
[needs-a6]: ../../../docs/research/terminal-rendering/consumer-needs.md#a6-width-detection
[needs-c]: ../../../docs/research/terminal-rendering/consumer-needs.md#part-c-needs-matrix
[needs-d2]: ../../../docs/research/terminal-rendering/consumer-needs.md#d2-hand-rolled-gaps-that-the-library-could-obviously-offer
[research]: ../../../docs/research/terminal-rendering/README.md
[rich-flaws]: ../../../docs/research/terminal-rendering/rich-and-typer.md#92-known-flaws-and-the-fix-for-each
[charm-weak]: ../../../docs/research/terminal-rendering/charm.md#72-weaknesses-visible-in-the-code
[layout-comfy]: ../../../docs/research/terminal-rendering/rust-layout-crates.md#3-comfy-table-801
[layout-ports]: ../../../docs/research/terminal-rendering/rust-layout-crates.md#10-rich-ports-on-cratesio
[layout-width]: ../../../docs/research/terminal-rendering/rust-layout-crates.md#72-unicode-segmentation
[layout-measure]: ../../../docs/research/terminal-rendering/rust-layout-crates.md#11-does-any-crate-separate-measure-from-render-the-way-rich-does
[style-auto]: ../../../docs/research/terminal-rendering/rust-style-crates.md#31-what-autostream-detects-and-whether-a-library-can-drive-it
[style-depth]: ../../../docs/research/terminal-rendering/rust-style-crates.md#53-color-depth
[presentation]: ../../../docs/presentation.md
[render-paint]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L662
[render-wrap]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/render.rs#L609
[layout-detect]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/layout.rs#L16
[style-rs]: https://github.com/victor-software-house/ctl-core/blob/7dc8060e2578af605f4ae858947693cb52477d6f/src/style.rs
[verctl-members]: https://github.com/victor-software-house/verctl/blob/41757015205f335135dfc84274b0330bcb9be4b0/src/members.rs
[m-a]: ../../../docs/research/terminal-rendering/measurements.md#matrix-a-one-binary-per-crate
[m-a3]: ../../../docs/research/terminal-rendering/measurements.md#a3-styling
[m-a4]: ../../../docs/research/terminal-rendering/measurements.md#a4-width-and-wrap
[m-b]: ../../../docs/research/terminal-rendering/measurements.md#matrix-b-ctl-core-tiers
[m-c]: ../../../docs/research/terminal-rendering/measurements.md#matrix-c-consumers
[pr-101]: https://github.com/victor-software-house/ctl-core/pull/101
[bencher-size]: https://bencher.dev/docs/how-to/track-file-size/
[sizewatcher]: https://github.com/adobe/sizewatcher
[cargo-size]: https://crates.io/crates/cargo-size
[cargo-bloat]: https://crates.io/crates/cargo-bloat
[cargo-bloat-action]: https://github.com/orf/cargo-bloat-action

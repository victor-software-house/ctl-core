# Design

## Context

ctl-core draws every human view of the ctl CLIs: records, lists, notices,
help. It does so in three passes over the same bytes
([consumer needs, A.5][needs-a5]):

1. `Renderer::paint` writes SGR escapes into each span before layout, for
   `--color auto` as well as `always` ([render.rs:662][render-paint]).
2. comfy-table measures and wraps those strings. To do so, its
   `custom_styling` feature parses the escapes again through `ansi-str` and
   `console`. Plain paragraphs go through a one-cell comfy table too
   ([render.rs:609][render-wrap]). The terminal width comes from
   `comfy_table::Table::width()`, which queries crossterm about `/dev/tty`
   ([layout.rs:16][layout-detect]).
3. `anstream::AutoStream` parses the escapes a third time to strip them when
   the stream is not a terminal.

`custom_styling` turns on comfy-table's `tty` feature, which links crossterm.
The research behind this design is in
[docs/research/terminal-rendering][research]. It covers:

- Python's rich and typer;
- Charm's lipgloss and glamour;
- 13 Rust layout crates and 7 Rust ports of rich;
- 19 Rust styling and terminal crates;
- every consumer's call sites;
- binary-size measurements of each candidate.

The approved crate-structure rules apply: the `rust-projects` skill, section 4
items 19 and 20 and section 5 items 4, 9 and 11. In short:

- a crate starts where a heavy dependency or an independent consumer starts;
- the facade keeps the family's bare name;
- leaves are `ctl-<noun>`;
- every crate in the workspace releases at one version;
- no generic code crosses a crate boundary.

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
5. The widgets rich has that the consumers already hand-roll. They are
   per-column width policy with ellipsis and numeric alignment, sanitized
   spans, tree, panel, hyperlinks, live progress. The last three come in later phases (decision
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
   ([consumer needs, D.2][needs-d2]).
   - rawr wrote its own escape-sequence filter and wrapper.
   - Private consumers, checked but not named here, wrote their own styled
     text, width rule and table.
   - A module inside ctl-core gives them clap, serde and the CLI chassis as
     well.
2. **A Cargo feature of ctl-core.** Same reach problem, and features unify
   across a build and spread `#[cfg]`. The approved rules make a crate the
   default way to make a piece opt-in.
3. **Its own repository**, released separately. It lost on lockstep.
   - Cargo treats every `0.0.x` as incompatible with every other. Each
     engine release would need a ctl-core release before any consumer could
     move.
   - A change that touches the facade and the engine would be two pull
     requests in two repositories.
   - Today every consumer reaches the engine through ctl-core. The crate
     boundaries in decision 3 are what a later move would need. The
     repository can split without renames once the engine has a consumer
     that does not use ctl-core.
   - verctl already releases a virtual workspace as one package and
     publishes its members in dependency order
     ([verctl members.rs][verctl-members]). The release lane stays one
     Version PR.
4. **Adopt an existing crate.**
   - comfy-table has no configuration that keeps styled cells without
     crossterm ([layout survey, 3][layout-comfy]).
   - tabled rewrites cells to wrap them and parses ANSI behind a feature
     that changes its width answer.
   - The rich ports are young. The most used has 6 releases. The most
     literal ships its own width tables and has 174 open issues
     ([layout survey, 10][layout-ports]).
   - None separates measuring from rendering for arbitrary nested content
     except the ports.
   - Measured costs are in decision 16.

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
| `src/verbatim.rs` (from [CTC-032][ledger]) | `ctl-text` | |
| `src/document.rs`, `src/render.rs` mapping half, `src/view.rs`, `src/help.rs`, `src/app.rs`, `src/flags/`, `src/parser.rs`, `src/color.rs`, `src/format.rs`, `src/model.rs` | stay in `ctl-core` | |
| `src/input.rs`, `src/input/`, `src/validate.rs` | stay | `ctl-input`: serde-saphyr, yamled, garde and toml are heavy dependencies a CLI may want without the chassis |
| `src/served.rs`, `src/surface.rs`, `src/surface/`, `src/usage.rs` | stay | `ctl-surface`: MiniJinja |
| `src/fixture.rs` | stay | `ctl-fixture`: a dev-only consumer with its own dependency, tempfile |

Alternatives:

1. **A `crates/` folder** holding every member, as BeautifulRust's
   M-CRATES-FLAT-FOLDER suggests. It lost: the house rule names a crate's
   folder like its package at the root. No measurement or source shows a
   gain from the extra level.
2. **Move everything now.** It lost on scope: each later move has its own
   consumers to migrate and its own evidence to collect. Folding them in
   would make this change unreviewable.
3. **Keep a single crate and put the engine in a second repository.** It
   lost under decision 1.

### 3. Three leaves, split where a dependency or an independent consumer starts

- `ctl-text`
  - Holds: styled text (`Span`, `Line`, `Text`), the cell-width model,
    wrapping and truncation, the unsafe-character set, and the renderable
    protocol (`Renderable`, `Measurement`, `Context`).
  - Depends on: `anstyle`, `unicode-width`, `unicode-segmentation`,
    `unicode-bidi`.
  - Independent consumer: rawr's title filter and wrapper; private
    consumers, checked but not named here.
- `ctl-layout`
  - Holds: widgets built on the protocol: `Table`, `Record`, `Rule`,
    `Stack`, `Indent`, later `Panel`, `Tree`, `Columns`, `ProgressBar`.
  - Depends on: `ctl-text`.
  - Independent consumer: private consumers, checked but not named here.
- `ctl-term`
  - Holds: terminal facts (`Terminal`: is attached to a terminal, colour
    depth, width), colour policy, the escape encoder, the `Console` writer,
    later `Live` and prompts.
  - Depends on: `ctl-text`, `anstyle`, and `terminal_size` behind the
    default `detect` feature.
  - Independent consumer: rawr's terminal check; private consumers, checked
    but not named here.
- `ctl-core`
  - Holds: the facade and the CLI chassis: `App`, flags, `Document`, the
    `Theme` that maps roles to styles, `View`, help, JSON.
  - Depends on: the three leaves behind features.
  - Independent consumer: every CLI.

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
lost. It has no dependency or consumer of its own, so the rules keep it a
module.

How the two rules are reconciled. BeautifulRust prefers small single-purpose
crates in a flat workspace (Microsoft M-SMALLER-CRATES and
M-CRATES-FLAT-FOLDER). The `rust-projects` skill splits only at a boundary
shown by a dependency, a second consumer or a measurement. Where they
conflict, the skill wins, because a split with no evidence is the boundary
Wasmtime merged back.

Workspace-wide versions and lints, and individual re-exports, come from
BeautifulRust and apply in full; its `crates/` folder does not (decision 2).

So `Table`, `Tree` and `Panel` share `ctl-layout`: they share every
dependency and no consumer wants one without the others. A widget that brings
a heavy dependency becomes its own crate: `ctl-markdown` with
`pulldown-cmark`, in a later phase.

Alternatives:

1. **One engine crate.** It lost because `ctl-term` brings `rustix` (and
   `windows-sys` on Windows) and the text consumers want neither. It also
   lost because it would make the pure layout code depend on the operating
   system.
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
   reuse.
   - anstyle is `Copy` and `const`, has no dependency and is stable at 1.0.
     It is what clap's `Styles` and cargo already use, so a CLI shares one
     type with its parser.
   - clap_builder links anstyle unconditionally, so it costs a ctl CLI
     nothing.
   - Tri-state inheritance is only needed for markup, which is a non-goal.
2. **owo-colors or yansi.** They write escapes at format time and decide
   colour from global state, which is the opposite of decision 5.

### 5. Escape sequences are written once, by the sink

Layout returns `Vec<Line>`, each line a list of `Span { text, style, link }`.
No escape byte exists before `ctl-term`'s encoder. The encoder:

1. writes an SGR sequence only where the style changes between spans, at the
   colour depth the terminal supports (decision 11);
2. skips a span with no text, and ends every line with no style open. A reset
   follows the last styled span. None follows a line that already ended in
   plain text, so each line stands alone in `head`, `less -R` and a log;
3. opens and closes OSC 8 around a linked span when links are on;
4. writes plain text, byte for byte, when colour is off.

So `anstream`, `anstyle-parse`, `utf8parse`, `colorchoice`, `ansi-str`,
`ansitok`, `nom`, `vte` and `console` leave the graph. The encoder needs no
terminal: its depth is an argument. ctl-core's string rendering therefore
calls the same encoder instead of keeping a second one (decision 19 states
what `ctl-term` supports).

Alternatives:

1. **Paint first, strip later**, as today and as lipgloss does on every
   operation ([Charm survey, 7.2][charm-weak]). It lost: it costs a parser
   and makes captured output depend on the strip. Layout has to measure
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
   when text is built. Layout never measures a tab.
4. Storage offsets are bytes into `str`. Widths are cells. The two units
   never share a type: a width is `u16`, an offset is `usize`.
5. A cut that falls inside a 2-cell cluster replaces the cluster with a space
   on each side, as rich does. Every line keeps its exact width.
6. `ctl_text::UNICODE_VERSIONS` states both crates' Unicode versions, and a
   test pins them, so an upgrade that moves one forces a recheck (the
   complete-sets rule). Today both are Unicode 17.0.0, with `unicode-width`
   locked at 0.2.2; 0.2.3 is on Unicode 18 ([layout survey, 7.2][layout-width]).

Alternatives:

1. **Sum of per-`char` widths** (what comfy-table and the prototype do). It
   lost: it miscounts every emoji sequence and splits clusters on a fold.
2. **rich's special cases** (extend over ZWJ and VS16 only). It lost under the
   complete-sets rule: regional indicators, Hangul jamo and Indic conjuncts
   are left to chance.
3. **No segmentation, break only at whitespace.** It would save the
   segmentation tables (5,248 bytes of `__text`, [measurements, A4][m-a4]).
   But a table that must fold a long word would still need cluster
   boundaries.

### 7. Text from outside the program is filtered when a span is built

`Span::new` drops every character `unsafe_chars::is_unsafe` refuses. These are
the C0 and C1 controls including ESC, the twelve UAX #9 directional formatting
characters, and the line and paragraph separators. It also splits on newline
and expands tabs.

So a commit subject or a session title cannot move the cursor, reorder the
line or break the layout, in any widget. No consumer has to filter by hand.

The set and its sources are the module of [CTC-032][ledger], which moves into
`ctl-text` unchanged. `Document::verbatim` keeps its rule (no wrapping,
newlines and tabs kept).

Alternative: **filter only in `verbatim`**, as today. It lost: rawr, the agent
runtime and its terminal client each wrote their own filter
([consumer needs, D.2][needs-d2], gap 5). Also, rich's equivalent strips only 5
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
4. The trait is dyn compatible. Containers hold `Box<dyn Renderable>`, so no
   generic code crosses from `ctl-layout` into its callers.

Alternatives: **ratatui's constraint solver.** It lost: it solves user
constraints for nested UI regions and never measures content
([layout survey, 11][layout-measure]). **papergrid's estimate-then-draw.** It
covers tables only. rich's split covers any nesting and is what the ports
copy.

### 9. The table chooses column widths in six steps

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
6. Narrow each column to its widest rendered line, as comfy-table does.
   - A column that wrapped otherwise keeps blank cells down every row that do
     nothing for the reader.
   - The lines stay as rendered, since text wrapped at one width wraps the
     same at its own widest line.
   - A width never grows. A keep-words word that runs past still does, and an
     `Ellipsis` column that cut a cell keeps its width because the cut cell
     fills it.

This fixes two flaws found in rich ([rich survey, 9.2][rich-flaws], flaw 10).
Its collapse ignores minimum widths, and its last step can shrink fixed and
no-wrap columns. Step 3 keeps the prototype's useful rule that a column whose
content fits its share takes its content width.

Step 6 was added after the parity corpus showed the solved width left on a
wrapped column. `qctl status` at width 80 drew 80 columns where comfy-table
drew 72. A table sized to its content reads as finished. The snapshot
tests of verctl, qctl, forkctl, and rawr change less when they take the
release.

Alternative: **keep the width step 3 gave a wrapped column.** It lost for the
reasons step 6 gives.

Alternative: **port comfy-table's dynamic arrangement.** It lost because it
measures strings with escapes in them. The result cannot be described in
a spec: parity snapshots (decision 17) show where the new algorithm differs.

### 10. Terminal facts are a pure function of an environment snapshot

`Terminal::detect(stream, mode, options, env)` returns `{ attached, color,
width }`. `env` is a lookup function, so a test passes a map and no test
touches process state. `Terminal::from_process(stream, mode, options)` passes
the process environment, reading it with `std::env::var_os` so that a value
that is not valid UTF-8 still counts as set.

`Terminal::from_probe(probe, mode, options, env)` is the whole decision with
the stream stated. `options` is `DetectOptions`, and there is one constructor
for each way to supply the input, not a pair with and without options.

Colour, for `ColorMode::Auto`, first match wins:

1. `NO_COLOR` set and non-empty: no colour.
2. `CLICOLOR_FORCE` set, non-empty and not `0`: colour.
3. `FORCE_COLOR` set, non-empty and not `0` or `false`: colour.
4. `CLICOLOR=0`: no colour.
5. `TERM=dumb`: no colour.
6. The stream is a terminal: colour. Otherwise none.

`FORCE_COLOR=0` and `FORCE_COLOR=false` are neutral: they neither force colour
nor turn it off, so rules 4 to 6 still apply. rich treats `FORCE_COLOR=0` as a
force, which is the flaw this avoids.

`ColorMode::Always` and `Never` skip rules 1 to 6. They still read the
variables that set the depth (`COLORTERM`, `TERM`, `FORCE_COLOR`) and the
width (`COLUMNS`). The decision is made once, before rendering, for the stream
the output goes to.

Width, first match wins:

1. an explicit width (exact);
2. the window size of the stream being written, through
   `terminal_size::terminal_size_of`;
3. `COLUMNS`;
4. the fallback (`DEFAULT_FALLBACK_WIDTH`, 80).

The column buffer (`DEFAULT_COLUMN_BUFFER`, 2) and the minimum
(`DEFAULT_MINIMUM_WIDTH`, 20) apply to a detected width. The buffer reads no
variable until the caller names some with
`DetectOptions::column_buffer_envs`. ctl-core passes its
`CTL_CORE_COLUMN_BUFFER` names and uses these constants rather than its own
copies.

A width is at least 1: an explicit width of 0 counts as 1. Help printed to
stderr now queries stderr, not stdout ([consumer needs, A.6][needs-a6]).

A probe of a stream is `Probe::Redirected` (a pipe, a file, or a closed
stream) or `Probe::Terminal` with the window width. `Probe::terminal(columns)`
builds it, and a width of 0 counts as not reported.
Everything that asks the operating system (`Probe::of`, `Terminal::detect`, the
`from_process` constructors, `Console::print`, and `terminal_size`) sits behind
the default `detect` feature, so ctl-core's `render` tier takes the encoder
without it.

Alternatives:

1. **anstream's detection.** It does not read `FORCE_COLOR`, re-detects on
   every print, and is tied to its writer ([style survey, 3.1][style-auto]).
2. **crossterm's size.** It reads `/dev/tty`, not the stream, so it is wrong
   for redirected output, and it links `parking_lot` and `mio`.
3. **rich's order.** It reads the width from stdin first and treats
   `FORCE_COLOR=0` as force ([rich survey, 9.2][rich-flaws], flaws 12 and
   13).

### 11. Colour depth is detected and every colour is downgraded to it

Depth:

- `COLORTERM` of `truecolor` or `24bit`, or `FORCE_COLOR=3`, is 24-bit;
- `TERM` containing `256color`, or `FORCE_COLOR=2`, is 256;
- anything else with colour on is 16.

The encoder maps a 24-bit colour to the nearest of the 256 palette. It maps a
256 colour to the nearest of the 16. Both mappings are pure functions. At 16
colours the underline colour is dropped, because SGR 58 takes a palette or
24-bit colour and has no 16-colour form.

No surveyed crate does this ([style survey, 5.3][style-depth]). The default
theme uses only the 16 named colours, so its output is the same at every
depth.

### 12. Roles map to styles through a public theme

`ctl_core::Theme` maps each `Role` to an `anstyle::Style`. `Theme::default()`
is today's palette ([style.rs][style-rs]). `App::theme`, `View::theme` and
`RenderOptions::theme` replace it; `ColorMode::Never` disables it. The theme
is applied when `Document` becomes widgets, so `ctl-layout` never sees a role.

### 13. A closed pipe ends the program quietly

The maintainer chose exit code 0 on 2026-11-08 (UTC). When stdout is closed
by the reader (`| head`), `Console` reports `BrokenPipe`, and `App` exits with
code 0 and writes nothing to stderr. Today `App` exits with a failure, so a
consumer must catch the error by hand.

The alternative, exit 141 as a shell does on SIGPIPE, was considered. It lost
because a ctl CLI is not killed by the signal. A non-zero exit also fails
`set -o pipefail` scripts for a reader that had enough. A consumer that
wants the error sets `App::broken_pipe(BrokenPipe::Fail)`.

A closed pipe never turns a failure into success.

Under the default, `BrokenPipe::Ignore`, `App` exits with the code the command
would have had with every byte read. A failed command keeps 1, and a parse
error keeps 2.

`BrokenPipe::Fail` turns only success into a failure and keeps a code that is
already non-zero. So a closed pipe never hides a more specific code. A script
that checks `$? -eq 2` behaves the same under both policies. Only a command
that succeeded exits 0.

Exiting 0 on every closed pipe was the first version. Review found it hid a
failed command whose error went to a reader that stopped early. A script
would then read it as a pass.

### 14. Phases: replace first, then add what rich has

- Phase 1 (this change), row [CTC-034][ledger]:
  - the workspace, the three leaves, `Table` with per-column overflow and
    numeric alignment, `Record`, `Rule`, `Stack`, `Indent`, paragraphs and
    headings;
  - the encoder, detection, `Console`, the theme, closed-pipe handling;
  - ctl-core on the new engine;
  - comfy-table and anstream removed.
- Phase 2, new row: `Panel`, `Tree`, `Columns`, `Align`, hyperlinks (OSC 8
  with an allow-list and `FORCE_HYPERLINK`), a `Table` footer and title.
- Phase 3, new row: `Live` region, `ProgressBar`, `Spinner`, `Status`,
  confirmation and secret prompts.
- Phase 4, new row: `ctl-markdown`: the bundled instructions rendered
  instead of printed raw.

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
They mark what a column holds, not how it is drawn.

A CLI that needs a widget `Document` does not have uses the engine through
the facade's re-exports (`ctl_core::layout`, `ctl_core::term`,
`ctl_core::text`) or the leaf crates directly:

```rust
use ctl_core::layout::{Column, Overflow, Table};
use ctl_core::term::{Console, DetectOptions, Stream};
use ctl_core::text::{Text, anstyle::AnsiColor};

let console = Console::from_process(Stream::Stdout, ColorMode::Auto, DetectOptions::new());
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

Rules every public item follows:

- no `bool` parameter (an enum instead);
- a builder method converts its argument at once and calls a concrete inner
  function, so its generic shim stays small;
- `#[non_exhaustive]` on public enums;
- `Debug` on every public type;
- `# Errors` on every fallible function.

### 16. Feature tiers and their measured sizes

ctl-core's features turn on leaves:

| ctl-core feature | Leaves | New compared with today |
|:--|:--|:--|
| `document` | none | unchanged |
| `render` | `ctl-text`, `ctl-layout`, `ctl-term` without `detect` | renders to a `String` with the one encoder; no window query, no `rustix` |
| `help`, `view` | turn on `ctl-term`'s `detect` | detection and the writer |
| `app` | as today | unchanged |

Measured on darwin-arm64 at [2f23725], the `main` commit CI verified in
[run 42542525587][ci-2f23725]. The baseline is ctl-core 0.6.x (task 7.2,
2026-11-08 UTC).

- The host was an Apple M4 Pro with macOS 26.7.1 and rustc 1.99.1.
- A strip check passed on every binary: `nm -U` lists no Rust symbol.
- File size is `stat -f %z`.
- `__text` is the `Section __text` line of `size -m`.
- Built packages are the lines of
  `cargo tree -e normal,build --target aarch64-apple-darwin`.

The toy is `ctl-core/examples/size.rs` in a crate of its own, with clap's
`std` and `derive` only. It uses opt-level `"s"`, fat LTO, one codegen unit,
abort and strip, and is built against v0.6.28 and against [2f23725]:

| Tier | File, 0.6.28 | File, main | `__text`, 0.6.28 | `__text`, main | Change | Built packages |
|:--|--:|--:|--:|--:|--:|--:|
| `cli` | 468,624 | 402,256 | 361,096 | 299,080 | -62,016 | 23 → 16 |
| `help` | 685,760 | 534,784 | 476,784 | 387,724 | -89,060 | 47 → 27 |
| `view` | 669,232 | 501,712 | 462,376 | 361,152 | -101,224 | 53 → 34 |
| `app` | 718,864 | 551,376 | 499,464 | 410,544 | -88,920 | 54 → 35 |

The toy's lock went from 65 packages, 63 from the registry, to 38 and 33.

The `cli` tier renders nothing, so its change is the narrowing of clap's
features alone, the figure [Matrix B][m-b] measured for it.

If the narrowing costs the other tiers what Matrix B measured, the engine's
share is about -40,500 (`help`), -41,000 (`view`) and -40,700 (`app`). The
prototype's share was -57,000. That split was not measured again on `main`.

The same example built inside this repository, with its own profile and lock,
is within 1,000 bytes of the toy at each tier.

The leaves' own dependencies cost little over a plain `println!` baseline:

- anstyle 1,620 bytes;
- unicode-width with unicode-segmentation 10,232;
- terminal_size 788;
- anstyle-query 596 (since removed, decision 19).

comfy-table as ctl-core 0.6.28 sets it costs 80,288, and crossterm's
`terminal::size` alone 23,396 ([measurements, Matrix A][m-a]).

Each public consumer was built at its `main` head with its own release
profile. One build was as committed. The other had an uncommitted
`[patch.crates-io]` to [2f23725], whose version was rewritten to the
consumer's pin:

| Consumer | Pin | File, as committed | File, patched | `__text`, as committed | `__text`, patched | Change |
|:--|:--|--:|--:|--:|--:|--:|
| [verctl][verctl-head] | 0.6.23 | 15,727,712 | 15,662,704 | 12,293,148 | 12,285,916 | -7,232 |
| [qctl][qctl-head] | 0.6.24 | 6,863,728 | 6,798,608 | 5,533,776 | 5,524,304 | -9,472 |
| [forkctl][forkctl-head] | 0.6.23 | 5,536,656 | 5,471,184 | 4,523,776 | 4,516,672 | -7,104 |
| [rawr][rawr-head] | 0.6.28 | 1,233,216 | 1,183,712 | 950,920 | 953,564 | +2,644 |

| Consumer | Lock packages | Built packages |
|:--|--:|--:|
| [verctl][verctl-head] | 408 → 397 | 315 → 309 |
| [qctl][qctl-head] | 219 → 212 | 167 → 162 |
| [forkctl][forkctl-head] | 138 → 126 | 119 → 110 |
| [rawr][rawr-head] | 121 → 115 | 94 → 89 |

The as-committed numbers equal [Matrix C][m-c] to the byte.

Every patched consumer drops ansi-str, ansitok, arrayvec, comfy-table,
console, minimal-lexical, nom and vte. It gains ctl-text, ctl-layout,
ctl-term and terminal_size. All but rawr also drop crossterm. All but qctl
drop unicode-general-category. forkctl drops parking_lot and the three
crates it brings.

The prototype had cut 28,524 to 45,632 bytes per consumer. The engine that
shipped is larger, and the attribution below shows where the difference
went. The private consumers were not measured.

`cargo bloat` 0.12.1 attributed `__text` to crates in qctl, the largest
cut, and rawr, which grows. Each one's release profile was left unstripped
and without debug info.

Four builds of each:

- A: as committed;
- B: patched to v0.6.28;
- C: patched to [2f23725];
- D: as C with the consumer's own clap dependency set to
  `default-features = false` (`std`, `derive`, and `env` for qctl).

Unstripped `__text`, in bytes:

| Step | qctl | rawr | What moved |
|:--|--:|--:|:--|
| A → B, ctl-core changes from the pin to 0.6.28 | -64 | +128 | nothing; rawr already pins 0.6.28, so its +128 is the noise floor |
| B → C, the engine and every other change to `main` | -8,256 | +2,580 | see below |
| C → D, the consumer's clap without default features | 0 | -60,808 | clap_builder -48,808, std -8,112, heck -5,984, strsim -1,864 |

In B → C:

- The engine crates come to 53,608 bytes in qctl and 52,872 in rawr. In
  qctl they are ctl-text 21,644, ctl-layout 24,296 and ctl-term 7,668. The
  grapheme tables of unicode-segmentation add 6,368 and 2,516.
- They replace comfy-table and the crates it brought, 51,016 bytes in qctl
  and 45,396 in rawr. Of these, comfy-table itself is 32,252 and 32,044. With
  the smaller crates, the swap nets +9,288 in qctl and +10,192 in rawr.
- The generic std code the removed crates instantiated shrinks by 11,228 in
  qctl and 2,620 in rawr. This turns qctl's total negative.
- ctl-core's own code, the other changes since 0.6.28 included, moves by -780
  in qctl and +784 in rawr. In rawr, 20,084 bytes leave ctl-core's crate and
  about as many reach rawr's, likely ctl-core code inlined into rawr. That was
  not checked.
- clap moves by -720 and -5,352.
- cargo-bloat attributes generic code by symbol name, so the rest, spread
  under 2 KB a crate, is not assigned to a cause.

qctl's C → D is zero because usage-lib turns clap's default features back
on. Its `clap` dependency leaves `default-features` on, in 6.1.1 and in
6.12.1, the latest ([usage-lib Cargo.toml][usage-lib-toml]). Every consumer
of ctl-core's `usage` feature (verctl, qctl and forkctl) carries clap's
`color`, `help`, `suggestions`, `error-context` and `usage`, whatever the
consumer requests.

What ctl-core could cut, measured and not changed:

1. **usage-lib's clap defaults.** rawr shows their cost: 60,808 bytes, 6.4%
   of its `__text`. For a consumer with `usage`, the cut needs usage-lib to
   take clap with `default-features = false`. The other way is for ctl-core
   to build its spec without usage-lib's `clap` feature. A consumer's own
   clap needs the same setting, as the crate's quick start says.
2. **The `usage` feature itself.** In qctl, kdl (117,572), usage-lib
   (108,540) and winnow (43,436) come to 269,548 bytes, 4.9% of its
   `__text`. Their job is to print one Usage spec for mise. Writing the KDL
   text straight from the clap command would drop all three.
3. **annotate-snippets** (103,692 in qctl) comes with serde-saphyr's
   `deserialize` feature ([serde-saphyr Cargo.toml][serde-saphyr-toml]),
   which ctl-core's `input` needs. ctl-core places its errors through
   yamled, but LTO keeps annotate-snippets, so some path still reaches it;
   which path was not checked.
4. **The largest ctl-core symbol**, the closure in `Renderer::widget`, takes
   9,328 bytes in qctl.

The engine itself stays: its three crates cost what the crates they replaced
cost, and its grapheme tables are the price of cluster-correct widths.

A budget task, `check:size`, builds the toy at each tier in CI on linux-x64.
It fails when `.text` grows more than 2% over that tier's budget.

The budgets are build configuration for this repository's own crates, so they
live where the repository already keeps tool configuration for its build.
That place is Cargo metadata, as `[package.metadata.docs.rs]` and
`[package.metadata.cargo-machete]` do. They form a `[workspace.metadata.size]`
table in the root `Cargo.toml`.

The task reads it with
`cargo metadata --format-version 1 --no-deps --locked | jq`, using the jq the
repository already pins. It measures `.text` with binutils' `size -A`, which
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
2. **Parity.** A corpus of the documents consumers render is taken from
   their snapshot and capture tests. ctl-core at the last comfy-table release
   renders it, and so does the new engine. The parity test commits both
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
   - colourless output contains no ESC byte; coloured output leaves no style
     open at the end of any line.
4. **Detection.** A table test of `Terminal::from_probe` over every variable in
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
   styled cells without a terminal crate and a second escape parser. The
   one that comes closest still ignores minimum widths. The algorithm is
   specified (decision 9) and property-tested (decision 17).
4. Replaced: the colour policy is written in `ctl-term` as a pure function.
   The reason is that no crate reads both `NO_COLOR` and `FORCE_COLOR` with an
   explicit override, per stream, without global state.

### 19. ctl-term supports and tests Unix only, for now

The maintainer chose this on 2026-11-08 (UTC). `ctl-term` has no Windows
path:

- it does not turn on virtual-terminal mode;
- it does not depend on `anstyle-query`;
- a target that is not Unix reports no window width.

The reason is what the Windows path did. `anstyle-query`'s
`windows::enable_ansi_colors` enables virtual-terminal mode for stdout and
stderr at once. A piped stdout therefore turned off the colour of a terminal
stderr, and mintty lost its colour.

No consumer ships a Windows build, and no CI job runs on Windows, so the path
could not be tested. The crate docs, the README and `ctl-term/AGENTS.md` state
it. Windows returns with a Windows CI job that runs the tests, and the path
comes back with them.

Alternatives:

1. **Enable virtual-terminal mode for the stream being written only.** It
   needs the Windows console API directly, which means `unsafe` or a new
   dependency, and no job could test it.
2. **Keep `anstyle-query` as it was.** It lost for the cross-stream defect
   above.

## Automatic behaviours

| Behaviour | Public override | Disable path |
|:--|:--|:--|
| Colour decided from the environment and the stream | `ColorMode::Always`, `--color always` | `ColorMode::Never`, `--color never`, `NO_COLOR` |
| Colour depth from `COLORTERM`, `TERM`, `FORCE_COLOR` | `Terminal::at_depth(depth)` | `ColorMode::Never` |
| Width from the stream, then `COLUMNS` | `width(n)` on `RenderOptions`, `View` | an explicit width |
| Buffer and minimum on a detected width | `DetectOptions::column_buffer`, `minimum_width` | a buffer or minimum of 0 |
| Buffer variable | `DetectOptions::column_buffer_envs` | none read by default |
| Fallback width 80 | `fallback_width(Some(n))` | `fallback_width(None)` |
| Unsafe characters removed from spans | none: it is a safety rule | `Document::verbatim` keeps tabs and newlines |
| Tabs expanded to 8 | `Context::tab_width(n)` | none |
| Closed pipe keeps the command's exit code | `App::broken_pipe(BrokenPipe::Fail)` | the same |
| Default theme | `App::theme`, `View::theme`, `RenderOptions::theme` | `ColorMode::Never` |

## Risks / Trade-offs

1. [Output differs from comfy-table in column widths] → the parity test
   (decision 17) lists every difference. Each consumer reviews its
   snapshot diff in the pull request that moves its pin.
2. [We now own a table algorithm] → it is specified in six steps and checked
   by properties. Its cases are a subset of rich's, whose behaviour is on
   record.
3. [`ctl_core::Text` (roles) and `ctl_text::Text` (styles) share a name] →
   the facade re-exports the leaf as a module (`ctl_core::text::Text`), never
   at the root. The docs show which is which.
4. [`unicode-segmentation` and `unicode-width` diverge on a new emoji] → the
   pinned version test fails on any upgrade that moves one of them.
5. [Four crates to release] → verctl publishes a virtual workspace as one
   package at one version, in dependency order.
6. [Captured output changes for `ColorMode::Auto`] → today a capture under
   `Auto` holds SGR bytes because the strip happens at write time. After this
   change, `Auto` off a terminal paints nothing. Consumer tests use `Never`
   ([consumer needs, A.5][needs-a5]), so none should break; the changeset
   states this.

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
[ledger]: ../../../tasks.yaml
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
[2f23725]: https://github.com/victor-software-house/ctl-core/commit/2f2372540862de53f1160ad247631a3ec1248ffb
[ci-2f23725]: https://github.com/victor-software-house/ctl-core/actions/runs/42542525587
[verctl-head]: https://github.com/victor-software-house/verctl/tree/d087741fd60190757141761eb8410a1a73875c53
[qctl-head]: https://github.com/victor-software-house/qctl/tree/cd06a58610646d6ebd9e2ec3ce2bdc1a7a59e4b7
[forkctl-head]: https://github.com/victor-software-house/forkctl/tree/7b987070f9069842c2e9afdd17fca28353b81bc5
[rawr-head]: https://github.com/victor-software-house/rawr/tree/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9
[usage-lib-toml]: https://docs.rs/crate/usage-lib/6.12.1/source/Cargo.toml
[serde-saphyr-toml]: https://docs.rs/crate/serde-saphyr/2.1.1/source/Cargo.toml
[bencher-size]: https://bencher.dev/docs/how-to/track-file-size/
[sizewatcher]: https://github.com/adobe/sizewatcher
[cargo-size]: https://crates.io/crates/cargo-size
[cargo-bloat]: https://crates.io/crates/cargo-bloat
[cargo-bloat-action]: https://github.com/orf/cargo-bloat-action

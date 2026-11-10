# Terminal rendering: research

Research for the [terminal-render-engine] change, collected 2026-11-07 (UTC).
Each survey is read from source code at a pinned version or commit, and says
what was not verified. The measurements were built and run.

| File | What it covers |
|:--|:--|
| [rich-and-typer.md] | Python's rich 15.0.0 and typer 0.28.0: the renderable protocol, styles, widths, ANSI, detection, every widget, typer's help panels, and rich's known flaws |
| [charm.md] | Go's lipgloss v2, glamour v2, fang v2, colorprofile and `x/ansi`: string-in, string-out styling and what it costs |
| [rust-layout-crates.md] | comfy-table, tabled and papergrid, ratatui, textwrap, unicode-width and unicode-segmentation, miette, cli-table, prettytable-rs, termimad, and seven Rust ports of rich |
| [rust-style-crates.md] | anstyle and anstream, owo-colors, yansi, nu-ansi-term, console, termcolor, supports-color, terminal_size, crossterm, indicatif, clap's styled help |
| [consumer-needs.md] | every call site of ctl-core's presentation layer in verctl, qctl, forkctl and rawr (private consumers were checked and are not named), what they hand-roll, and a needs matrix |
| [measurements.md] | binary size of each candidate crate, of ctl-core's tiers on comfy-table and on a prototype engine, and of each public consumer |

## What the research found

1. **Paint late.** rich renders to segments of text and style and writes
   escapes only in its final writer, so it never parses its own output.
   lipgloss and ctl-core paint first and parse the escapes again at every
   layout step ([rich, 9.1][rich-copy]; [Charm, 7.2][charm-weak];
   [consumer needs, A.5][needs-a5]).
2. **Measure, then render.** rich's renderables report a minimum and a maximum
   width before drawing, which lets a container choose good widths. Among
   Rust crates only the rich ports do this for nested content; papergrid does
   it for tables ([Rust layout, 11][layout-measure]).
3. **No Rust table crate keeps styled cells cheaply.** comfy-table needs its
   `tty` feature, and so crossterm, to keep a styled cell
   ([Rust layout, 3][layout-comfy]).
4. **The style type already exists.** `anstyle::Style` is `Copy`, has no
   dependency, is stable, and is what clap uses. No surveyed crate converts a
   24-bit colour down to 256 or 16 colours ([Rust style, 5.3][style-depth]).
5. **Width needs grapheme clusters.** rich only recently handled emoji
   sequences, and its segment splitting still approximates. `unicode-width`
   0.2 measures sequences, and `unicode-segmentation` gives cluster
   boundaries ([rich, 3.2][rich-graphemes]; [Rust layout, 7][layout-width]).
6. **Detection should be a value.** rich reads width from stdin first and
   treats `FORCE_COLOR=0` as force; anstream ignores `FORCE_COLOR` and
   re-detects on every print; crossterm asks `/dev/tty` instead of the
   stream ([rich, 9.2][rich-flaws]; [Rust style, 3][style-questions]).
7. **The consumers hand-roll what a library should give.** A table with
   ellipsis and numeric columns, a width rule, closed-pipe handling, a filter
   for untrusted text, a live view, prompts, a tree and hyperlinks
   ([consumer needs, D.2][needs-d2]).

The Rust style survey recommends anstream as the output adapter. The design
does not take it: once colour is decided before rendering, the adapter would
parse output in which it never finds an escape to strip (design, decision 5).

[terminal-render-engine]: ../../../openspec/changes/archive/2026-11-10-terminal-render-engine/proposal.md
[rich-and-typer.md]: rich-and-typer.md
[charm.md]: charm.md
[rust-layout-crates.md]: rust-layout-crates.md
[rust-style-crates.md]: rust-style-crates.md
[consumer-needs.md]: consumer-needs.md
[measurements.md]: measurements.md
[rich-copy]: rich-and-typer.md#91-what-to-copy
[rich-flaws]: rich-and-typer.md#92-known-flaws-and-the-fix-for-each
[rich-graphemes]: rich-and-typer.md#32-graphemes
[charm-weak]: charm.md#72-weaknesses-visible-in-the-code
[needs-a5]: consumer-needs.md#a5-pipeline-from-document-to-bytes
[needs-d2]: consumer-needs.md#d2-hand-rolled-gaps-that-the-library-could-obviously-offer
[layout-measure]: rust-layout-crates.md#11-does-any-crate-separate-measure-from-render-the-way-rich-does
[layout-comfy]: rust-layout-crates.md#3-comfy-table-801
[layout-width]: rust-layout-crates.md#7-unicode-width-023-and-unicode-segmentation-1133-as-they-affect-width
[style-depth]: rust-style-crates.md#53-color-depth
[style-questions]: rust-style-crates.md#3-the-three-questions

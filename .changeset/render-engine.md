---
ctl-core: minor
---

Render every document through the workspace's own engine in place of comfy-table and anstream.
The engine is three new crates on crates.io, `ctl-text`, `ctl-layout`, and `ctl-term`, published at the workspace version.
`ctl_core::{text, layout, term}` re-export them for a CLI that needs a widget directly.
comfy-table, anstream, and unicode-width are no longer dependencies.

Breaking changes:

- `kv`, `grid`, and the `ctl_core::table` module are removed.
  Compose `Fields` or `Table` inside a `Document` instead.
- `Document::render` no longer asks a terminal for its width or colour.
  It lays out to the explicit width, then the fallback width, then the natural width.
  It writes colour only under `ColorMode::Always`.
  `View`, help, and errors still decide colour and width for the stream they write.
- A tab in a verbatim block expands to the next 8-cell stop.
- Table layout changes in a few places, so snapshot tests of command output may change:
  - an empty column is 2 cells wide, where it was 3;
  - columns shrink widest first, and a framed table folds only a word wider than its column;
  - two words never run past the width;
  - colour no longer changes the layout, and every line that writes a style ends with a reset.

  A wrapped column still narrows to its widest line, as before.
- A reader can close stdout or stderr early, as `head -1` does.
  `App` then exits with the code the command would have had and writes nothing to stderr.
  In 0.6 it exited 1.
  - A command that succeeded exits 0.
    A failed command or a parse error keeps its own code, so a closed pipe never turns a failure into success.
  - `App::broken_pipe(BrokenPipe::Fail)` makes every closed pipe a failure: success becomes exit 1, and a failure keeps its own code.

Added:

- `Theme` maps each role and notice label to an `anstyle::Style`.
  `RenderOptions::theme`, `View::theme`, and `App::theme` replace the default.
- `Table::ellipsis_column` marks a title or summary column.
  A cell too wide for it keeps one line and ends in `…`, and the column gives up width before a column of whole words does.
- `Table::numeric_column` aligns a column, header included, to the right.

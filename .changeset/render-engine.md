---
ctl-core: minor
---

Render every document through the workspace's own engine (`ctl-text`, `ctl-layout`, and `ctl-term`) in place of comfy-table and anstream. comfy-table, anstream, and unicode-width are no longer dependencies, and `ctl_core::{text, layout, term}` re-export the three crates for a CLI that needs a widget directly.

Breaking changes:

- `kv`, `grid`, and the `ctl_core::table` module are removed. Compose `Fields` or `Table` inside a `Document` instead.
- `Document::render` no longer asks a terminal for its width or colour. It lays out to the explicit width, then the fallback width, then the natural width, and writes colour only under `ColorMode::Always`. `View`, help, and errors still decide colour and width for the stream they write.
- A tab in a verbatim block expands to the next 8-cell stop.
- Table layout changes in a few places. An empty column is 2 cells wide, where it was 3. Columns shrink widest first, and a framed table folds only a word wider than its column. Two words never run past the width. Colour no longer changes the layout, and every line that writes a style ends with a reset. A wrapped column still narrows to its widest line, as before. Snapshot tests of command output may change.
- When the reader closes stdout or stderr early, as `head -1` does, `App` exits with the code the command would have had and writes nothing to stderr, where it exited 1. A command that succeeded exits 0; a failed command or a parse error keeps its own code, so a closed pipe never turns a failure into success. `App::broken_pipe(BrokenPipe::Fail)` makes every closed pipe a failure: it turns success into exit 1 and keeps a failure's own code.

Added:

- `Theme` maps each role and notice label to an `anstyle::Style`. `RenderOptions::theme`, `View::theme`, and `App::theme` replace the default.
- `Table::ellipsis_column` marks a title or summary column: a cell too wide for it keeps one line and ends in `…`, and the column gives up width before a column of whole words does. `Table::numeric_column` aligns a column, header included, to the right.

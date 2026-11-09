---
ctl-core: minor
"verctl:group": changed
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
- `ListStyle` is `Frame`, and `RecordStyle`, `Frame`, and `RowSeparation` are the types of `ctl-layout`, so each is `#[non_exhaustive]` and `Hash`.
  `JsonLayout` is `#[non_exhaustive]` too.
  `RenderOptions::list_style` is `frame`; the variable `CTL_CORE_LIST_STYLE` keeps its name.
- Every setter on `RenderOptions` and `View` takes the name of its setting: `width`, `record_style`, `quiet`, and so on.
  `RenderOptions` has no getter, because a getter would need the same name.
  `View` reads its format and quiet setting through `format()` and `is_quiet()`.
  A method that reads the environment starts with `resolved_`.
  - `record()`, `list()`, and `separation()` are `resolved_record_style()`, `resolved_frame()`, and `resolved_row_separation()`, and `View::layout()` is `resolved_json_layout()`.
  - The getters `active_theme`, `fallback`, `explicit_width`, `explicit_automatic_width_buffer`, `automatic_width_buffer_env_names`, and `automatic_width_minimum` are gone.
    Their settings are the setters `theme`, `fallback_width`, `width`, `automatic_width_buffer`, `automatic_width_buffer_envs`, and `minimum_automatic_width`.
  - `View` holds one `RenderOptions` and returns it from `render_options()`.
    `View::color(mode)` sets the colour, and the colour is not a public field.
  - `App` keeps its setter names.
- `Stream` in the view module is `Destination`, so it no longer shares a name with `ctl_term::Stream`.
- `Renderer` is crate-private. `Document::render` renders to a string.
- `Document::render` and the view write through the `ctl-term` encoder, so a line ends with a reset only when a style is still open.
- The buffer and minimum settings, `automatic_width_buffer` and the like with their `DEFAULT_COLUMN_BUFFER*` and `DEFAULT_MINIMUM_AUTOMATIC_WIDTH` constants, exist only with `view` or `help`.
  Without a stream, nothing reads them.
- `parser::assert_help_enabled` is crate-private.

Added:

- `Theme` maps each role and notice label to an `anstyle::Style`: `role_style(role)` reads one and `style(role, style)` sets one.
  `RenderOptions::theme`, `View::theme`, and `App::theme` replace the default.
- `Table::ellipsis_column` marks a title or summary column.
  A cell too wide for it keeps one line and ends in `…`, and the column gives up width before a column of whole words does.
- `Table::numeric_column` aligns a column, header included, to the right.

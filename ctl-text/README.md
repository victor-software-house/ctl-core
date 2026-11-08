# ctl-text

Styled text for terminals: spans that are safe to print, widths counted in
terminal cells, and lines wrapped or cut to fit.

`ctl-text` lays text out as plain data before anything is written to a
terminal.
A `Span` is text in one style, and a `Line` is a list of spans.
A `Text` is a list of lines that wraps to a width.

None of them holds an escape sequence, so text from outside the program cannot
move the cursor or reorder a line.
A commit subject is an example.
Whatever writes the lines to a terminal adds the styling after layout.

The crate also provides:

- `text_width`, `cluster_width`, and `prefix_len`, which count cells per
  extended grapheme cluster.
- `Overflow`, the policy for a line wider than its width.
  It keeps words whole, folds inside a word, or ends with an ellipsis.
- `Renderable`, `Context`, and `Measurement`, the protocol a widget follows.
  A widget reports its size, then draws itself at a width.
- `verbatim::is_unsafe`, the rule for which characters never reach a terminal.

## Example

```rust
use ctl_text::anstyle::AnsiColor;
use ctl_text::{Context, Overflow, Renderable, Style, Text};

let green = Style::new().fg_color(Some(AnsiColor::Green.into()));
let text = Text::styled("build finished with no warnings", green).overflow(Overflow::KeepWords);

let lines = text.render(&Context::new(15));
let plain: Vec<String> = lines.iter().map(ToString::to_string).collect();
assert_eq!(plain, ["build finished", "with no", "warnings"]);
```

## Features

`ctl-text` has no optional features.

## Documentation

The API reference is on [docs.rs](https://docs.rs/ctl-text).

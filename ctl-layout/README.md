# ctl-layout

Terminal widgets built on [ctl-text](https://docs.rs/ctl-text): tables, records, rules, stacks, and indents, and a way to swap one widget for another by width.

A widget measures the narrowest and the natural width it can be drawn at, then renders lines at the width it is given.
The lines are plain styled spans with no escape sequence in them.
Whatever writes them to a terminal adds the styling.

## What it provides

A table gives each column the widest cell it holds.
When the width runs short, it shrinks the widest columns first.
A grid keeps its frame aligned by folding words that no longer fit.

| Widget | Draws |
|:--|:--|
| `Table` | Rows under column headers, in a grid, under one header rule, or plain |
| `Record` | Labelled values, with right-aligned keys, left-aligned keys, or a box |
| `Definitions` | Entries as a line of labels over indented lines of description, the narrow form of a table |
| `Breakpoint` | One widget below a width and another from it up |
| `Rule` | A horizontal line with an optional title |
| `Stack` | Widgets one above the other, with optional blank lines between |
| `Indent` | A widget moved right by a number of cells |

## Example

```rust
use ctl_layout::{Column, Context, Frame, Overflow, Renderable, Table};

let table = Table::new([
    Column::new("id"),
    Column::new("title").overflow(Overflow::Ellipsis),
    Column::new("age").numeric(),
])
.frame(Frame::HeaderRule)
.row(["job-0014", "Fix the layout of narrow tables", "3"])
.row(["job-0015", "Document each public item", "120"]);

let lines: Vec<String> = table
    .render(&Context::new(40))
    .iter()
    .map(ToString::to_string)
    .collect();
assert_eq!(
    lines,
    [
        "id        title                      age",
        "────────────────────────────────────────",
        "job-0014  Fix the layout of narrow…    3",
        "job-0015  Document each public item  120",
    ]
);
```

Every widget implements `Renderable`, so a widget of your own sits in a `Stack` beside these.
A `Stack` and an `Indent` take a widget by value or by reference.
ctl-layout re-exports `ctl_text`, `Context`, `Overflow`, `Renderable`, and `Text`.

## Features

ctl-layout has no optional features.

## Documentation

The API reference is on [docs.rs](https://docs.rs/ctl-layout).

## License

MIT.

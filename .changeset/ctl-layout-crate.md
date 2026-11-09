---
ctl-core: patch
---

New crate on crates.io: `ctl-layout`, terminal widgets built on `ctl-text`.

- A `Table` takes `Column`s with an `Overflow` and an `Align` each, a `Frame` (`Grid`, `HeaderRule`, or `Plain`), and a `RowSeparation` (`None`, `Rule`, or `Blank`).
- Each column gets its widest cell.
  When the width runs short, the widest column shrinks first, never below the widest word of a keep-words column.
  A framed table folds instead of running past the width.
- A numeric column aligns right, headers included.
- `Definitions` draws each entry as a line of labels over indented description lines.
  `Breakpoint` draws one widget below a width and another from it up, so a table gives way to definitions on a narrow terminal.
- `Record`, `Rule`, `Stack`, and `Indent` complete the set.
  A `Stack` and an `Indent` take a widget by value or by reference.
  Every widget implements `ctl_text::Renderable`, so it measures before it renders.
- ctl-layout re-exports `ctl_text`, `Context`, `Overflow`, `Renderable`, and `Text`.

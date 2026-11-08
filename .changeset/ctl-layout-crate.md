---
ctl-core: patch
---

New crate on crates.io: `ctl-layout`, terminal widgets built on `ctl-text`.

- A `Table` takes `Column`s with an `Overflow` and an `Align` each, a `Frame` (`Grid`, `HeaderRule`, or `Plain`), and a `RowSeparation` (`None`, `Rule`, or `Blank`).
- Each column gets its widest cell.
  When the width runs short, the widest column shrinks first, never below the widest word of a keep-words column.
  A framed table folds instead of running past the width.
- A numeric column aligns right, headers included.
  `stacked_below(threshold, label_columns)` renders each row as indented lines below the threshold.
- `Record`, `Rule`, `Stack`, and `Indent` complete the set.
  Every widget implements `ctl_text::Renderable`, so it measures before it renders.

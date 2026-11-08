---
ctl-core: patch
---

Add the `ctl-layout` crate to the workspace: terminal widgets built on `ctl-text`. A `Table` takes `Column`s with an `Overflow` and an `Align` each, a `Frame` (`Grid`, `HeaderRule`, or `Plain`), and a `RowSeparation` (`None`, `Rule`, or `Blank`). It gives each column its widest cell, shrinks the widest column first when the width runs short, never below the widest word of a keep-words column, and folds a framed table instead of letting it overflow. A numeric column aligns right, headers included, and `stacked_below(threshold, label_columns)` renders each row as indented lines under the threshold. `Record`, `Rule`, `Stack`, and `Indent` complete the set, and every widget implements `ctl_text::Renderable`, so it measures before it renders. ctl-core does not use the crate yet.

---
ctl-core: patch
---

Add the `ctl-text` crate to the workspace: styled text for terminals over `anstyle::Style`. A `Span` removes every character `verbatim::is_unsafe` refuses, a `Line` is spans, and `Text` splits at each newline and expands tabs to the next tab stop. Widths are counted in terminal cells per grapheme cluster (`cluster_width`, `text_width`, `prefix_len`), and `UNICODE_VERSION` states the Unicode versions of the width and segmentation data. A line is fitted to a width by `Overflow`: `KeepWords` breaks at spaces only, `Fold` breaks a long word between clusters, and `Ellipsis` cuts the line with `…`. No rendered line ends in a space. Widgets implement `Renderable` with `Measurement` and `Context`, and `overlay` combines two styles. `ctl_core::verbatim` is now a re-export of `ctl_text::verbatim`: its path, its items, and its `verbatim` feature are unchanged, and the feature now enables the `ctl-text` dependency in place of `unicode-bidi`.

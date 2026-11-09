---
ctl-core: patch
---

New crate on crates.io: `ctl-text`, styled text for terminals over `anstyle::Style`.

- A `Span` removes every character `verbatim::is_unsafe` refuses.
  A `Line` is a row of spans, and `Text` splits at each newline and expands tabs to the next tab stop.
- Widths count terminal cells per grapheme cluster (`cluster_width`, `text_width`, `prefix_len`).
  `UNICODE_VERSION` names the Unicode version of the width and segmentation data.
- `Overflow` fits a line to a width: `KeepWords` breaks at spaces only, `Fold` breaks a long word between clusters, and `Ellipsis` cuts the line with `…`.
  No rendered line ends in a space.
- Widgets implement `Renderable` with `Measurement` and `Context`, and `overlay` combines two styles.

`ctl_core::verbatim` re-exports `ctl_text::verbatim`.

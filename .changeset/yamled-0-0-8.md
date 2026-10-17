---
ctl-core: patch
---

ctl-core edits YAML with yamled 0.0.8, which writes one-line text in the document's quotes instead of keeping a quoted node's old quotes. ctl-core's own behaviour does not change: it reads positions from yamled and does not write through it.

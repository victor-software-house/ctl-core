---
ctl-core: patch
---

ctl-core reads YAML with serde-saphyr 2.0.0 and edits it with yamled 0.0.7. `Limits::budget()` now returns serde-saphyr 2.0's `Budget`; a caller that passes it to serde-saphyr 1.x must move to 2.0 with it. The `comments` feature is on, so the comment limits in `Limits` still apply.

An error raised inside an aliased value keeps serde-saphyr's own wording and points at that value, followed by where the anchor is defined. ctl-core no longer parses positions out of the rendered alias text.

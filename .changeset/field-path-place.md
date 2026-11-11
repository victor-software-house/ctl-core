---
ctl-core: patch
---

`Input::place` turns a `FieldPath`, the steps down to a field, into the line and column where it is written, and `Input::problem_at` builds a placed `Problem` from one.
A CLI places a problem of its own with them, as the loader places garde's.
`FieldPath::from(&garde::Path)` reads garde's path parts, so a problem under a key that holds `.`, `*` or `/`, such as a glob `docs/**/*.md`, lands on that key's line and not on its parent's.
`Problem::field` is still garde's printed path.
A YAML `Input` parses its text once for placing, where each failing `check` parsed it again.
garde is now pinned to exactly 0.23.0, because the conversion reads a doc-hidden item of its `Path`.

---
ctl-core: patch
---

A CLI can report its own refusals of an input in ctl-core's format.
`InputError::whole(name, message)` refuses the whole input, on no line.
`InputError::try_new(name, problems)` carries a list of `Problem`s, each placed with `Input::place` or on no line, and returns `None` for an empty list.
Both print as ctl-core's own problems do, one per line as `name:line:column: field: message`.

---
ctl-core: patch
---

Rename the two public types whose names held "kind", and the field and method that use them.
This breaks every caller that names them:

- `WarningKind` is now `FlagClash`, and `FlagWarning`'s `kind` field is now `clash`.
  A match on `warning.kind` becomes a match on `warning.clash`.
- `MessageKind` is now `Outcome`, and `Present::message_kind` is now `Present::outcome`.
  A result that overrode `message_kind` overrides `outcome`, with the same `Success` and `Error` variants.

Each variant and `Outcome::default_exit_code` keep their names, and no output changes.

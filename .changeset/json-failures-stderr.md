---
ctl-core: minor
---

Every JSON failure now goes to stderr, and stdout stays empty after a failed run.
That covers `View::emit_err`, a parse error, a handler error, and a result whose outcome is an error.
A caller that read the error envelope from stdout reads it from stderr instead.

`ErrorBody` gains an optional `kind`, which JSON leaves out when it is unset, and `ErrorBody::kind` sets it.
A parse error's envelope sets `kind` to `usage`.

A handler returns the new `CommandError` to give a failure its own exit code and `kind` value.
`App` finds it anywhere in the `anyhow` chain, as the error or as context on its cause.
A plain `anyhow` error still exits 1 with no `kind`.

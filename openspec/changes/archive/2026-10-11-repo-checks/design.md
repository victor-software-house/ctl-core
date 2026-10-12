# Design: repository checks in the shared verify

## Decisions

### 1. A required `check:repo`, not an override

`verify` depends on `check:repo` by name. Each repository defines it, and a
repository with nothing to add defines a task that does nothing.

The alternative was to keep overriding `verify` in each repository. It lost
because every override repeats the shared list, and a task added to the
shared `verify` then never runs in a repository that overrides it.

Making `check:repo` optional was also considered. mise has no optional
dependency, so a missing task fails `verify`, which is the safer failure.

Override: a repository can still define its own `verify`, which replaces
the shared one. Disable: a repository that removes the include loses every
shared task.

### 2. ctl-core's own `check:repo` runs `true`

ctl-core has no checks outside the shared tasks. Its `check:repo` runs
`true` so the shared `verify` resolves in ctl-core too.

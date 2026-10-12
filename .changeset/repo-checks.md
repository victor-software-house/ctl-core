---
ctl-core: patch
---

The shared `verify` in `tasks/rust.toml` now also depends on `check:repo`, which each including repository defines for its own checks, in place of overriding `verify`. A repository that moves its include to this release must define `check:repo` (a task that runs `true` when it has no checks of its own), or `verify` fails with `task not found: check:repo`.

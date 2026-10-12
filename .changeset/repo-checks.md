---
ctl-core: patch
---

The shared `verify` in `tasks/rust.toml` now also depends on `check:repo`, which each including repository defines for its own checks, in place of overriding `verify`.

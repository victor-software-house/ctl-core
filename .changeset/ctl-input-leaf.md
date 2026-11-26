---
ctl-core: patch
---

Declared input now lives in the new `ctl-input` crate, which `ctl_core::input` re-exports.
The `input-yaml`, `input-toml`, `validate`, and `input` features and every path under `ctl_core::input` are unchanged, so no code that uses them changes.

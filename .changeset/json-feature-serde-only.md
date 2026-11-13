---
ctl-core: patch
---

The `json` feature enables only `serde`, and `view` enables `serde_json` with it.
Only `view` writes JSON, so `json` alone builds no `serde_json`.

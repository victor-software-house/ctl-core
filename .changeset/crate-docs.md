---
ctl-core: patch
---

docs.rs shows ctl-core as a landing page: a summary, a compiled quick start, how output flows, and the Cargo features, with each gated item labelled by its feature. It lists the modules and the prelude once. The crate-root re-exports are hidden from the docs but still resolve, so no import changes. Every fallible public function documents its errors.

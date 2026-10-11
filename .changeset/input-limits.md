---
ctl-core: patch
---

`Input::parse` now enforces safety limits for every tool: a text over 2 MiB is refused before parsing, and a document must be the only one, nested at most 32 deep, with at most 100,000 nodes and 2 MiB of scalar text. The limits are public as `input::MAX_BYTES`, `MAX_DEPTH`, and `MAX_NODES`. Anchors and aliases stay readable; duplicate keys, merge keys, and loose booleans stay refused.

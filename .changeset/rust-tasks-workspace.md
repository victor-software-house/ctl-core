---
ctl-core: patch
---

The shared `lint`, `test` and `test:doc` tasks pass `--workspace`, so a workspace with a root package or a `default-members` list checks every member.

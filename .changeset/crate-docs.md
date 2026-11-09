---
ctl-core: patch
---

The docs.rs pages for `ctl-core`, `ctl-text`, `ctl-layout`, and `ctl-term` now open on a landing page that starts with a quick start and ends with the Cargo features.

`ctl-core`'s page lists its modules and the prelude, so each type appears once.
It names where to start with declared input, which feature reads each format, and the renamed `FlagClash` and `Outcome`.

The `ctl-term` README example now renders to a string, so it shows no hidden line on GitHub.
No API changes.

---
ctl-core: patch
---

The `input` feature reads YAML through serde-saphyr 2.1.1, its latest release.
serde-saphyr reports the same problems in the same words, and allocates less while it parses.
The lock moves granit-parser, its YAML parser, from 2.0.0 to 2.1.0 as well.

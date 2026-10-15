---
ctl-core: patch
---

The shared `test` task runs `cargo test --lib --tests` instead of `--lib --bins --tests`. `--bins` selects every binary target and ignores its `test` flag, so a fuzz target declared with `test = false` was built and run as a test. `--tests` selects every target whose `test` flag is on, so bins keep their unit tests.

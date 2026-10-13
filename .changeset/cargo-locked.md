---
ctl-core: patch
---

The shared Rust tasks pass `--locked` to clippy, both test tasks, cargo-deny, and the toolchain check's `cargo metadata`, so `verify` fails when `Cargo.lock` no longer matches `Cargo.toml`. Before, a stale lock passed `verify` and failed only a release build that passes `--locked`. `deps` runs a locked `cargo metadata` before cargo-machete, whose own unlocked metadata call rewrote a stale lock while sibling tasks ran. A repository passes `--locked` in its own `check:features` too.

---
ctl-core: patch
---

The shared Rust tasks pass `--locked` to clippy, both test tasks, and cargo-deny, so `verify` fails when `Cargo.lock` no longer matches `Cargo.toml`. Before, a stale lock passed `verify` and failed only a release build that passes `--locked`. A repository passes `--locked` in its own `check:features` too.

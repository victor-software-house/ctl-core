---
ctl-core: patch
---

New feature `baseline`: `baseline::render(root, &config)` returns the Rust configuration every ctl repository shares (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `lefthook.yml`, and the marked `lints` and `tools` blocks in `Cargo.toml` and `mise.dev.toml`) as `served::Served` values, from a repository's `.ctl/rust.yaml` (`baseline::load`).

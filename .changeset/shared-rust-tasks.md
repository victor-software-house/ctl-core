---
ctl-core: patch
---

ctl-core now ships the standard Rust mise tasks (`check:toolchain`, `format`, `lint`, `test`, `test:doc`, `deny`, `deny:advisories`, `deps`, `semver`, `verify`) in `tasks/rust.toml`. Another repository includes it with `git::https://github.com/victor-software-house/ctl-core.git//tasks/rust.toml?ref=<tag>` in `task_config.includes`, and defines its own `check:features`.

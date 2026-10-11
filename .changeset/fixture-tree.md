---
ctl-core: patch
---

New `fixture` feature for integration tests: `Suite::new(name)` and `suite.scenario(name)` each own a temporary directory that is removed when it drops, and `root.file(path, body)` writes a fixture with its parent directories in one call. A path outside the root panics instead of writing.

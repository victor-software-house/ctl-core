---
ctl-core: patch
---

Remove the `go`, `run`, `main`, `main_with`, and `main_with_help` wrappers. `App` is the only entry point. The `cli` feature no longer pulls in `anyhow`; `app` does.

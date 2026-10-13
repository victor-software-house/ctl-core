---
ctl-core: patch
---

New `mount_directives(task)`: the `#MISE raw=true`, `#MISE raw_args=true`, and `#USAGE mount` block a served CLI-proxy task carries. Without `raw`, mise relays the task's output and appends an SGR reset to every line whenever its own colour is on, as under `CLICOLOR_FORCE=1`, so `--format json` piped to `jq` carried an ESC byte the CLI never wrote.

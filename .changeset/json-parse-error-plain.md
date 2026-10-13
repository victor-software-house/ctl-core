---
ctl-core: patch
---

A parse error under a long `--format json` or `--format=json` is now a JSON error even when the bad argument comes before the flag. Clap's lenient read stopped at the bad argument, so the error printed as text, coloured when `CLICOLOR_FORCE` or `FORCE_COLOR` was set, to a caller that asked for JSON.

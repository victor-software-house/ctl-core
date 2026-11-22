---
ctl-core: patch
---

A parse error now reads `<bin>: <message>`, without Clap's `error:` prefix, in pretty output and in the JSON envelope's `message`.

`OutputArgs`, `FormatArgs`, and `FormatLong` gain `--json`, a shorthand for `--format json` that sets their `format` field.
Naming `--json` beside a `--format` other than `json` is a usage error.
With `env_prefix`, `--json` reads `<PREFIX>_JSON`, either flag on the command line wins over the other's variable, and `<PREFIX>_JSON` wins over `<PREFIX>_FORMAT`.
A CLI that calls `Surface::require_shorts` adds `--json` to its allow list, since `--json` has no short form.
A parse error after `--json` is JSON, as it is after a long `--format json`.

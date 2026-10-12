---
ctl-core: patch
---

`input::Limits` overrides any of serde-saphyr's `Budget` fields except `enforce_alias_anchor_ratio`, which stays on, decided field by field: the call (`Input::limits`), then the repository (`limits:` in `.ctl/input.yaml`, installed by `input::use_repo_limits`), then the process (`input::set_default_limits` or `App::input_limits`), then serde-saphyr's default. `max_reader_input_bytes` now also caps text already in memory, a limit error names the limit, its value, and where to raise it, and `use_repo_limits` returns each limit the repository raises above serde-saphyr's default for the CLI to print as a warning.

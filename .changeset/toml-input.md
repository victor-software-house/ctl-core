---
ctl-core: patch
---

New `input-toml` feature: the declared-input loader reads TOML. `Input::open(path)` takes the format from the extension (`.toml`, `.yaml`, `.yml`) and refuses any other extension; `Input::format(Format)` sets it for text in memory, and `Input::new` and `Input::read` stay YAML, so existing consumers change nothing. A TOML parse error, a serde error, and each garde field problem (a top-level key, a table, an inline table, an array of tables) report a 1-based line and column, and a TOML text is held to `max_reader_input_bytes`, `max_depth`, and `max_nodes` from the same tiers as YAML. The `input` feature keeps meaning YAML and compiles no TOML crate.

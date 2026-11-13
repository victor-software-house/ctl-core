# Read a YAML tag through the declared input

## Why

hakkiri marks a project setting that beats every other one with the YAML tag `!override`, as in `code_span_size: !override 0`.  
A CLI reads its config only through [`Input`][input], and `Input` exposes no tag.  
serde-saphyr captures one with `Tagged<T>`, but 0.0.3 read a tagged node into an untagged enum as the one-key map `{override: value}`, so `Tagged<T>` lost the value for hakkiri's settings.  
The fork patch `tagged-plain-value` fixes that in vsh-serde-saphyr 0.0.4.  
The queue row is [CTC-060][ledger].

## What Changes

- `ctl_core::input::Tagged` re-exports serde-saphyr's `Tagged<T>`, so a CLI wraps a config field in it and reads the field's tag beside its value.
- ctl-core requires vsh-serde-saphyr 0.0.4.
- An input with no `Tagged` field parses as before, byte for byte.

Nothing is **BREAKING**: the re-export is new, and 0.0.4 changes only what `Tagged<T>` gives its inner type.

## Capabilities

### Modified Capabilities

- `declared-input`: a config field keeps its YAML tag.

## Impact

- Changed: `Cargo.toml`, `Cargo.lock`, `ctl-core/src/input.rs`, `ctl-core/AGENTS.md`, `ctl-core/README.md` and the `declared-input` spec.
- A test in `ctl-core/tests/input.rs`.
- No new dependency: the fork moves from 0.0.3 to 0.0.4.

[input]: ../../../ctl-core/src/input.rs
[ledger]: ../../../tasks.yaml

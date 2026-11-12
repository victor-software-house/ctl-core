# Place a key as well as a value

## Why

`Input::place` returns where the value at a path starts.
For a scalar or a flow value that is the line of its key, but for a block mapping or a block list it is the first line below the key.
A CLI whose fault is about the key itself, such as a glob that is no glob or a setting that names no rule, cannot put the fault on the key.
The queue row is [CTC-054][ledger].

yamled holds the span of each key, and the `toml` tree holds the span of each key too, so both can answer.

## What Changes

- A public `Input::place_key(&FieldPath) -> Option<Place>`: where the last step of the path is written as a key, or where the nearest written ancestor's key is.
- An item of a list or array and the root have no key, so they give the place of their value, as `place` does.
- It shares the limits verdict and the cached yamled document of `place`, so it adds no parse and no new limits path.

Nothing is **BREAKING**: the method is new, and `place` does not change.

## Capabilities

### Modified Capabilities

- `declared-input`: an input places a key.

## Impact

- Changed: `ctl-core/src/input.rs`, `input/yaml_text.rs`, `input/toml_text.rs`, `ctl-core/AGENTS.md`, `ctl-core/README.md` and the `declared-input` spec.
- Tests in `tests/input.rs`, `tests/input_toml.rs` and `tests/input_toml_alone.rs`, and in `yaml_text.rs`.
- No new dependency.

[ledger]: ../../../tasks.yaml

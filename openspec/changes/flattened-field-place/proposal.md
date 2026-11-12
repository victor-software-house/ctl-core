# Place a problem under a flattened field where the file writes it

## Why

serde's `#[serde(flatten)]` puts the entries of a Rust field directly under its parent in the file.  
garde still names the field in each problem's path.

verctl's `assets` holds its targets in a flattened map field `targets` ([verctl #313][verctl-313]).  
A problem in `assets.linux-x64.files` reaches [`Input::check`][input] as `assets.targets.linux-x64.files`.  
The file has no `targets` key, so the problem lands on the `assets:` line, and the printed field names a key the reader cannot find.

garde_derive 0.23 parses `#[garde(rename)]` but never emits it, so the schema cannot drop the segment itself.  
The queue row is [CTC-049][ledger].

## What Changes

- A public `Input::flattened(FieldPath) -> Self`: the schema field at that path is flattened, so the file holds its entries directly under its parent.
- `Input::check` drops that field's step from each garde path that passes through it, before it places the problem and before it prints the field.
- The rule acts on the path, so YAML and TOML place the same way.
- A problem whose path passes through no declared field keeps its printed field and its place, byte for byte.

Nothing is **BREAKING**: the method is new, and an input that declares nothing checks as before.

## Capabilities

### Modified Capabilities

- `declared-input`: a schema declares a flattened field, and its problems sit where the file writes them.

## Impact

- Changed: `ctl-core/src/input.rs`, `ctl-core/AGENTS.md`, `ctl-core/README.md` and the `declared-input` spec.
- Tests in `tests/input.rs` and `tests/input_toml.rs`.
- No new dependency.
- verctl declares `assets.targets` once it pins the release that carries this.

[input]: ../../../ctl-core/src/input.rs
[ledger]: ../../../tasks.yaml
[verctl-313]: https://github.com/victor-software-house/verctl/pull/313

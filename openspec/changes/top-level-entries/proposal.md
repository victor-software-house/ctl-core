# List the top-level entries of a YAML or TOML text

## Why

hakkiri lints the string values of a Markdown file's front matter.
It needs, for each top-level entry, the key, the byte range of the value, how the value is written and, for a string, its text.
Its first version read the keys with a line scanner of its own, which skipped anchors, aliases and complex keys, and the stack that carried it was reverted for that reason.

ctl-core reads the same front matter for hakkiri's config already, through [`Input`][input].
serde-saphyr resolves anchors, aliases and tags there, and yamled indexes the spans.
yamled 0.0.13 lists a mapping's entries with `Node::entries` and finds the entries after a key that is no scalar, which the line scanner could not.
The belonging queue row is [CTC-055][ledger].

## What Changes

- A public `Input::top_level() -> Result<Vec<Entry>, InputError>` lists the entries of the root mapping or table in the order written.
  An `Entry` holds the key, the `value` byte range, a `Written` style and the `string` the parser reads, if any.
- A YAML input types each value through serde-saphyr under the limits in force, takes the spans from the yamled document that `Input::place` keeps, and shares both with `place`.
- A TOML input checks the text through `Input::parse`, then reads the root table of the `toml` tree, whose keys and values carry their spans.
- A root that is no mapping lists nothing, and a key that is no string is left out.
- yamled moves to 0.0.13.

Nothing is **BREAKING**: every item is new.

## Impact

- ctl-core `input` module, the workspace `Cargo.toml` (yamled and the `input` and `input-toml` size budgets), `ctl-core/AGENTS.md`, the crate README and the `declared-input` spec.
- Consumers: hakkiri takes the method for its front matter after the release.

[input]: https://github.com/victor-software-house/ctl-core/blob/481bba1/ctl-core/src/input.rs
[ledger]: ../../../tasks.yaml

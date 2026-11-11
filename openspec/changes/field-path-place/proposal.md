# Place every problem through one public field path

## Why

`Input::check` turns each garde path into its printed string, then splits the string on `.` and `[` to find the steps down to the field ([`field_steps`][field-steps]).
A mapping key that holds a dot reads as nesting, so a problem under the key `docs/**/*.md` is placed on the key's parent.
The code says so in its own doc comment: a key holding `.` or `[` "cannot be told apart from nesting".

The same string split is the only way ctl-core places anything, and it is private.
A CLI whose own checks run after the read, such as hakkiri's glob checks, cannot ask ctl-core for a line.
hakkiri scans the text for the key from the glob's line, with `key_line` in its `config.rs`.

garde keeps the steps: its `Path` holds a kind and a key for each component, which `Display` flattens to the string ctl-core splits ([garde `error.rs`][garde-path]).

The belonging queue row is [CTC-053][ledger].
It also touches [CTC-049][ledger], which starts from the same printed path.

## What Changes

- A public `FieldPath`: a list of steps, each a mapping or table key or a sequence or array index.
  It is built with `FieldPath::root().key(..).index(..)`, and from garde's `Path` with `From<&garde::Path>` under `validate`, by reading the path's parts and never its printed string.
- A public `Input::place(&FieldPath) -> Option<Place>`: the 1-based line and column where the path is written, or where its nearest written ancestor is, with a frontmatter offset applied.
- A public `Input::problem_at(&FieldPath, message) -> Problem`, so a CLI builds a placed problem for its own check.
- `Input::check` places each garde problem through `place`.
  `Problem.field` stays garde's printed string, for display only.
- The yamled document of a YAML input is parsed on the first placing and kept, so `check` and later calls to `place` on one `Input` parse it once.
- `garde` is pinned to `=0.23.0`, because the path parts come from a doc-hidden item.
- Removed: `field_steps` and the printed-string split.

Nothing is **BREAKING**: every item is new, and the placing of a path with no dotted key does not change.

## Impact

- ctl-core `input` module, `Cargo.toml`, `README.md` of the `ctl-core` crate where it names placing, `ctl-core/AGENTS.md`, and the `declared-input` spec.
- Consumers: none change.
  hakkiri's switch from its text scan to `place` is its own row.
- Binary size: the `input` and `input-toml` tiers grow by a few hundred bytes each; the budgets in `[workspace.metadata.size]` are measured again.

## Left open

CTC-049 stays open for what this change does not do:

1. A garde path with a segment the file lacks, such as `assets.targets.linux-x64.files` under a flattened map, still stops at the last segment that resolves.
2. The printed field stays garde's string, not the path the file holds.

[field-steps]: https://github.com/victor-software-house/ctl-core/blob/1cbad5cf9888ac9f87d2c0cf9207b83e844df564/ctl-core/src/input.rs#L411-L428
[garde-path]: https://docs.rs/crate/garde/0.23.0/source/src/error.rs
[ledger]: ../../../tasks.yaml

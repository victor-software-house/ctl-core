# Design

## Context

`Input::check` validates with garde, prints each problem's path to a string, and gives the strings to a format's `places`.
For YAML, `places` parses the text again with yamled and looks each string up after [`field_steps`][field-steps] splits it.
For TOML, `places` parses again with the `toml` crate and walks the tree by the same steps.
`places` is private, so no caller can place a problem of its own.

serde-saphyr and yamled sit on the same granit-parser release, 2.1.0 in the lock, so the two parses read keys at the same place.
A successful load parses once. The second parse happens only when problems exist, and today it happens again for every failing `check`.

## Goals

1. One path type from garde's path parts to the lookup, with no split of a printed string.
2. A public call that places a path, for a problem a CLI raises after the read.
3. The YAML document parsed at most once per `Input`.
4. `input-toml` alone still links no YAML crate and no garde.

## Non-goals

1. Skipping a path segment the file lacks, which is CTC-049.
2. Recording paths in the serde-saphyr read.
   That is a change to the fork and would make every load pay for it.
3. A cache for the TOML tree.
   It borrows the text, and a TOML config is small.

## Decisions

### 1. ctl-core owns `FieldPath`, not yamled

`FieldPath` is a list of `Step::Key(String)` and `Step::Index(usize)`.
yamled's `Path` has the same shape.
ctl-core still defines its own, because `input-toml` links no YAML crate by design, and the TOML walk needs the same steps.
The YAML side converts a `FieldPath` to a yamled `Path` with its builders.

Alternative: re-export yamled's `Path`.
It lost because a TOML-only build would need yamled for a type.

### 2. garde's path converts from its parts

`From<&garde::Path>` walks `Path::__iter()` in reverse, which is root first, and maps a key component to `Step::Key`, an index component to `Step::Index`, and drops a component with no key.
`__iter` and `Kind` are `#[doc(hidden)]`, so the change pins `garde = "=0.23.0"` and says why at the call.
A component of the index kind that is not a number is read as a key.

Alternative: garde's `serde` feature, which serializes a `Path` root first.
It lost because it turns on a feature of garde, and a consumer would pay for a serializer to read one list.

### 3. `place` returns the nearest ancestor

`place` follows the path as far as the file has it and returns where it stops, as `check` already did.
A caller that needs an exact hit compares the result with the place of the path's parent.
That keeps one rule for garde's problems and for a CLI's own.

Alternative: return `None` for a missing path and let the caller walk up.
It lost because every caller would write the walk, and garde's own placing already needs it.

### 4. The YAML document is built once and kept

`Input` holds a cell that fills on the first placing.
`Clone` copies the filled cell, `Debug` shows it, and equality ignores it, because the cell holds data derived from the text.
`check` and `place` read the same cell, so `load` followed by `check` parses the document once.

Alternative: parse in `check` only, as today.
It lost because hakkiri's `load` and `check` each failed and parsed again, and a public `place` would parse on every call.

The TOML tree is not kept.
It borrows the text, so keeping it would need a self-referential cell or an owned copy, and `check` places every garde problem with one parse anyway.

### 5. The result is a `Place`, built with a constructor

`Place` holds the 1-based line and column.
It is `#[non_exhaustive]` and built inside ctl-core, as the other public structs are, so a field such as an end position can join it.

### 6. `problem_at` fills `field` from the path

`problem_at` prints the path as garde prints it (`files.docs/**/*.md.vocab[1]`), so `Problem.field` reads the same for a CLI's problem and a garde problem.
The printed form is for people only, and nothing parses it.

## Risks

- A garde release changes `__iter`: the exact pin turns it into a deliberate bump, and a test of the conversion fails first.
- A consumer holds an alias inside a validated YAML document: yamled lands on the alias node, as it does today.

[field-steps]: https://github.com/victor-software-house/ctl-core/blob/1cbad5cf9888ac9f87d2c0cf9207b83e844df564/ctl-core/src/input.rs#L411-L428

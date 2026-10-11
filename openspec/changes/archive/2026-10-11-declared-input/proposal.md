# Proposal: declared input

Queue row: CTC-008 (first slice).

## Why

Each ctl CLI reads its config its own way. verctl parses YAML with
`yaml_serde` 0.10.7 and qctl with `serde_yml` 0.0.12, so two CLIs already
disagree on the parser. verctl carries four garde rules in its `schema.rs`
that qctl and forkctl would copy. A schema error names a field but no line.

## What Changes

- New `validate` feature: the four garde rules verctl built, unchanged:
  `inside_the_repo`, `at_least_one`, `cannot_be_empty`, `one_file_name`.
- New `input` feature: `Input` reads a file, parses its shape with
  `serde-saphyr`, validates it once with garde, and returns every problem
  with its file, line, column, field, and message.
- `Input::frontmatter` splits YAML frontmatter from a Markdown body, keeping
  the file's line numbers, so changeset fragments read through the same path.
- Dependencies, only under `input`: `serde-saphyr` 1.3.0 with only its
  `deserialize` feature, `yamled` 0.0.1 with default features off, and garde
  0.23.0 with default features off.

## Capabilities

### New Capabilities

- `declared-input`: validators, loading, error placement, and frontmatter.

### Modified Capabilities

None.

## Impact

- A patch release of ctl-core. Nothing existing changes.
- Later slices of CTC-008: verctl, qctl, and forkctl move onto `input` in
  their own repositories, verctl drops `schema.rs` and `gray_matter`, and the
  template renderer moves here.

# Tasks

## 1. The model

- [ ] 1.1 Make `ErrorBody` non-exhaustive and add `causes`, `problems`, `usage` and `parse` with their builders and schema.
  Verify: model tests serialize each field and leave each out when empty, and the schema test passes.

## 2. Filling it

- [ ] 2.1 Fill `parse` and `usage` for a parse error, with and without `error-context`.
  Verify: the two parse-error scenarios pass, one in a build with `error-context` and one without.
- [ ] 2.2 Fill `problems` for an `InputError` and `causes` for a handler error.
  Verify: the refused-input and causes scenarios pass.

## 3. Pretty, docs and release

- [ ] 3.1 Render the parts in the default `write_error`.
  Verify: the pretty scenario passes and the existing error snapshots change only where clap's message is more specific.
- [ ] 3.2 Update the rustdoc, `docs/presentation.md` and `ctl-core/AGENTS.md`, and write a `ctl-core: patch!` changeset naming each field.
  Verify: the doc and prose tasks pass in CI, and `openspec validate structured-errors --strict` passes.

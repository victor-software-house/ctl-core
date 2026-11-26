# Tasks

## 1. The model

- [ ] 1.1 Add to the non-exhaustive `ErrorBody` `causes`, `problems`, `usage` and `parse` with their builders and schema.
  Verify: model tests serialize each field and leave each out when empty, and the schema test passes.

- [ ] 1.2 Add ctl-core's `error-context` feature, which turns on `cli` and clap's `error-context`, and change the guidance in `ctl-core/AGENTS.md` that leaves `error-context` to the consumer: a consumer that wants a parse error's parts turns on ctl-core's feature.
  Verify: `check:features` builds the feature alone, and the scenario built with it passes.

## 2. Filling it

- [ ] 2.1 Fill `parse` and `usage` for a parse error, with and without `error-context`.
  Verify: the two parse-error scenarios pass, one in a build with `error-context` and one without.
- [ ] 2.2 Fill `problems` for an `InputError` and `causes` for a handler error.
  Verify: the refused-input and causes scenarios pass.

## 3. Pretty, docs and release

- [ ] 3.1 Keep the text a person reads beside the parts, and print it in the default `write_error`.
  Verify: every pretty error output is byte-identical to the base, in a build with ctl-core's `error-context` and one without.
- [ ] 3.2 Update the rustdoc, `docs/presentation.md` and `ctl-core/AGENTS.md`, and write a `ctl-core: patch` changeset naming each field.
  Verify: the doc and prose tasks pass in CI, and `openspec validate structured-errors --strict` passes.

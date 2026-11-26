# Tasks

## 1. The model

- [x] 1.1 Add to the non-exhaustive `ErrorBody` `causes`, `problems`, `usage` and `parse` with their builders and schema.
  Verify: model tests serialize each field and leave each out when empty, and the schema test passes.
  2026-11-26 UTC: [#347][pr347] added the fields, `InputProblem` and `ParseFailure` with their builders; the model and schema tests passed in CI.

- [x] 1.2 Add ctl-core's `error-context` feature, which turns on `cli` and clap's `error-context`, and change the guidance in `ctl-core/AGENTS.md` that leaves `error-context` to the consumer: a consumer that wants a parse error's parts turns on ctl-core's feature.
  Verify: `check:features` builds the feature alone, and the scenario built with it passes.
  2026-11-26 UTC: [#347][pr347] added the feature and changed the guidance; `check:features` builds it alone and with `app`, and the scenario built with it passed in CI.

## 2. Filling it

- [x] 2.1 Fill `parse` and `usage` for a parse error, with and without `error-context`.
  Verify: the two parse-error scenarios pass, one in a build with `error-context` and one without.
  2026-11-26 UTC: [#347][pr347] fills `parse` with one field per clap context, which a unit test maps for all 17, and `usage` on every parse error; both scenarios passed in CI, the second in the `test:error-context-off` task.
- [x] 2.2 Fill `problems` for an `InputError` and `causes` for a handler error.
  Verify: the refused-input and causes scenarios pass.
  2026-11-26 UTC: [#347][pr347] fills `problems` and `causes`; the refused-input scenario uses two garde problems, since an unknown key stops at the first, and the causes scenario passed in CI.

## 3. Pretty, docs and release

- [x] 3.1 Keep the text a person reads beside the parts, and print it in the default `write_error`.
  Verify: every pretty error output is byte-identical to the base, in a build with ctl-core's `error-context` and one without.
  2026-11-26 UTC: [#347][pr347] keeps the text on `ErrorBody`, out of JSON and the schema.
  27 error cases printed byte-identical stderr against the base on the build host, with and without `error-context`.
  The pretty snapshots from before the change pass in both builds in CI.
- [x] 3.2 Update the rustdoc, `docs/presentation.md` and `ctl-core/AGENTS.md`, and write a `ctl-core: patch` changeset naming each field.
  Verify: the doc and prose tasks pass in CI, and `openspec validate structured-errors --strict` passes.
  2026-11-26 UTC: [#347][pr347] updated the docs and carries `.changeset/structured-errors.md` as `ctl-core: patch`; the doc tasks passed in CI and `openspec validate structured-errors --strict` passes.

[pr347]: https://github.com/victor-software-house/ctl-core/pull/347

# Tasks

Archive [open-input](../open-input/proposal.md) before this change, because its `input-formats` capability is the one this change adds to.

## 1. The reader

- [x] 1.1 Add the scanner and string decoding in `ctl-input/src/json/`, behind the `json` feature, for every rule of design decision 4.
  Verify: unit tests for each rule pass on the build host.
  2026-11-26 UTC: [#340][pr340] added the scanner and string decoding with a unit test per grammar rule; CI passed.
- [x] 1.2 Add the loop that builds the located tree with an explicit stack, the shared limits, and the duplicate-key check of design decisions 2, 5, 7 and 8.
  Verify: tests for the span, duplicate and limit scenarios of the `input-formats` delta pass on the build host.
  2026-11-26 UTC: [#340][pr340] added the stack loop, the limits and the duplicate check, with the span, duplicate and limit scenarios as tests; CI passed.
- [x] 1.3 Vendor JSONTestSuite `test_parsing` at commit 1ef36fa with its license, and add the verdict test of design decision 11.
  Verify: the test reads all 318 files and passes on the build host, and a file left out of the verdict table fails it.
  2026-11-26 UTC: [#340][pr340] vendored the suite at 1ef36fa; the test reads all 318 files: 104 read, 213 refused, one read only from `max_depth` 500. CI passed.

## 2. The format

- [x] 2.1 Add `ScalarSyntax::Null`, the `JSON` format with its `resolve` of design decision 6, and `json` in `Formats::builtin()`.
  Verify: tests for the resolve scenarios pass, and the same field reads the same through YAML, TOML and JSON.
  2026-11-26 UTC: [#340][pr340] added `ScalarSyntax::Null`, `JSON` and its `resolve`, with a test that reads one field through YAML, TOML and JSON; CI passed.
- [x] 2.2 Add ctl-core's `input-json` feature, and leave `input` without it.
  Verify: `check:features` builds `input-json` alone, and a test shows `Input::open("app.json")` refused under `input` alone.
  2026-11-26 UTC: [#340][pr340] added `input-json`, its feature sets in `check:features`, and a test built with `input` alone that names the missing feature; CI passed.
- [x] 2.3 Add an `input-json` tier to `check:size`, measured on the build host, and leave every other tier's budget as it is.
  Verify: `check:size` passes in CI, and the pull request lists the new tier's measurement.
  2026-11-26 UTC: [#340][pr340] budgeted `input-json` at `app`'s budget plus the 72,097 bytes measured above `app`; `check:size` passed in CI, and no other budget moved.

## 3. Docs and release

- [x] 3.1 Update the crate docs of ctl-input and ctl-core, the feature docs in both manifests, `ctl-core/AGENTS.md` and the README for the JSON format and `input-json`.
  Verify: the doc and prose tasks pass in CI.
  2026-11-26 UTC: [#340][pr340] updated the crate docs, feature docs, `AGENTS.md` files and READMEs; the doc and prose tasks passed in CI.
- [x] 3.2 Write a `ctl-core: patch` changeset that names `input-json`, `input::JSON` and `ScalarSyntax::Null`.
  Verify: `openspec validate json-input --strict` passes, and the Version PR lists the entry.
  2026-11-26 UTC: [#340][pr340] carries `.changeset/json-input.md` as `ctl-core: patch`; `openspec validate json-input --strict` passes. The Version PR lists it once it refreshes.

[pr340]: https://github.com/victor-software-house/ctl-core/pull/340

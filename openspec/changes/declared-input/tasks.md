# Tasks: declared input

## 1. Library

- [x] 1.1 `validate` feature with the four rules from verctl. Verify:
  `cargo check --no-default-features --features validate` in
  `check:features` on the build host.
- [x] 1.2 `input` feature: `Input::{new, read, frontmatter, parse, check,
  load}`, `InputError`, `Problem`. Verify: `tests/input.rs` under
  `mise run verify` on the build host.

## 2. Release

- [x] 2.1 A patch changeset. Verify: `.changeset/declared-input.md`.

## Evidence

2026-09-29. `mise run verify` on the build host: 140 tests pass, including
`tests/input.rs` (a valid file loads; four schema problems reported with exact
lines and columns; loose booleans, duplicate keys, and merge keys refused;
frontmatter problems keep the file's line numbers). `check:features` compiles
`validate` and `input` alone.

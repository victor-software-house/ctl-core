# Tasks: declared input

## 1. Library

- [x] 1.1 `validate` feature with the four rules from verctl. Verify:
  `cargo check --no-default-features --features validate` in
  `check:features` on the build host.
- [x] 1.2 `input` feature: `Input::{new, read, frontmatter, parse, check,
  load}`, `InputError`, `Problem`. Verify: `tests/input.rs` under
  `mise run verify` on the build host.
- [x] 1.3 An alias error names where its value is defined, counted from the
  file, and repeats no position serde-saphyr wrote into its text. Verify:
  `an_alias_error_names_where_its_value_is_defined_in_the_file` under
  `mise run verify` on the build host.

## 2. Release

- [x] 2.1 A patch changeset. Verify: `.changeset/declared-input.md`.

## Evidence

2026-09-29. `mise run verify` on the build host: 140 tests pass, including
`tests/input.rs` (a valid file loads; four schema problems reported with exact
lines and columns; loose booleans, duplicate keys, and merge keys refused;
frontmatter problems keep the file's line numbers). `check:features` compiles
`validate` and `input` alone.

2026-09-29, task 1.3. `mise run verify` on the build host: 142 tests pass.
`an_alias_error_names_where_its_value_is_defined_in_the_file` fails on the
previous `Input::parse`, whose message read
`invalid u16 at line 2, column 9 (defined at line 2, column 9) at line 3, column 7 (defined at line 2, column 3)`:
serde-saphyr 1.3.0 nests one rendered alias error inside the next for each
alias level, with every position counted from the frontmatter.

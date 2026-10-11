# Tasks

- [x] 1. `Limits` with every `Budget` field, the four tiers, and the byte cap
  on in-memory text. Proof: `cargo check --no-default-features --features
  input` in `check:features` on the build host.
- [x] 2. Tests for each tier, an unset field falling through, the repository
  file, and the limit error. Proof: `tests/input_limits.rs` under
  `mise run verify` on the build host.
- [x] 3. AGENTS.md states the tiers. Proof: the Declared input section.
- [x] 4. A patch changeset. Proof: `.changeset/input-limits.md`.

## Evidence

2026-10-11 UTC. `mise run verify` passes on a build runner. The three tests in
`tests/input_limits.rs` cover: the depth error naming the file, line, limit,
value, and `.ctl/input.yaml`; an 18-byte in-memory text refused under
`max_reader_input_bytes: 10` with no line; and one sequence through every
tier. In it, serde-saphyr's default admits six levels, a process default of 4
refuses them, a repository's 8 admits them, a call's 5 refuses them, an unset
repository field falls through to the process's `max_nodes`, a missing
`.ctl/input.yaml` overrides nothing, an unknown field in it is refused by
name, and a tight repository installed first does not stop the next one
loading. The existing input tests pass unchanged, so the defaults behave as
before.

Not done here: forkctl calling `use_repo_limits` and reading through
`Input::parse`, which its session does.

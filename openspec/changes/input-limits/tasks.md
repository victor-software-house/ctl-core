# Tasks

- [x] 1. `Input::parse` enforces the size check and the budget; constants
  exported. Proof: `cargo check --no-default-features --features input` in
  `check:features` on the build host.
- [x] 2. Tests for each limit and for aliases. Proof: `tests/input.rs` under
  `mise run verify` on the build host.
- [x] 3. AGENTS.md states the limits. Proof: the Declared input section.
- [x] 4. A patch changeset. Proof: `.changeset/input-limits.md`.

## Evidence

2026-10-11 UTC. `mise run verify` passes on a build runner: 154 tests,
including five new in `tests/input.rs`: a text over 2 MiB refused before
parsing with no line, a second document refused, 33 nested mappings refused
and 31 read, 100,000 scalars refused and 1,000 read, and an alias read. Each
refused text is otherwise valid YAML, so only the limit refuses it. The
existing duplicate-key, merge-key, loose-boolean, and alias-position tests
still pass.

Not done here: forkctl moving onto `Input::parse`, which its session does.

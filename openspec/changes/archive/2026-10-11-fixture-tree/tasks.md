# Tasks

- [x] 1. `fixture` feature with `Suite`, `Suite::scenario`, `Root::file`, and
  `Root::read`. Proof: `cargo check --no-default-features --features fixture`
  in `check:features` on the build host.
- [x] 2. Behaviour tests. Proof: `tests/fixture.rs` under `mise run verify`
  on the build host.
- [x] 3. AGENTS.md names the feature. Proof: the feature list and the
  Fixtures section.
- [x] 4. A patch changeset. Proof: `.changeset/fixture-tree.md`.

## Evidence

2026-10-11 UTC. `mise run verify` passes on a build runner: 148 tests,
including the five in `tests/fixture.rs` (a nested file and its parents, two
scenarios that share nothing, a dropped scenario and a dropped suite removed,
and `..` or an absolute path refused without writing). A `compile_fail,E0505`
doctest on `Suite::scenario` shows a scenario cannot outlive its suite. `check:features`
compiles `fixture` alone. Its one warning, an unused `serde_json` under
`--features json`, predates this change.

Not done in this slice: verctl, qctl, and forkctl moving onto `fixture`. That
is the row's last acceptance line and happens in those repositories.

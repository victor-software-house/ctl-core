# Tasks

- [x] 1. `served` feature with `render`, `Served`, `Served::write`, and
  `ServeError`. Proof: `cargo check --no-default-features --features served`
  in `check:features` on the build host.
- [x] 2. Behaviour tests ported from verctl's renderer cases. Proof:
  `tests/served.rs` under `mise run verify` on the build host.
- [x] 3. AGENTS.md names the feature. Proof: the feature list and the Served
  files section.
- [x] 4. A patch changeset. Proof: `.changeset/served-files.md`.

## Evidence

2026-10-11 UTC. `mise run verify` passes on a build runner: 158 tests,
including the nine in `tests/served.rs`, ported from verctl's renderer cases
(targets from declarations, refused declarations naming the template and the
field, an unknown export, the shebang and trailing newline, an undefined name,
a missing suffix, ctl-core fragments, the 0755 and 0644 modes with a stale
mode corrected, and a symlinked target). `check:features` compiles `served`
alone. `Cargo.lock` gains no crate.

Not done here: verctl moving onto `served`, which happens inside its redesign.

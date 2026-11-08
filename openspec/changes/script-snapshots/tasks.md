# Tasks

- [x] 1. Plan: this change, and the CTC-031 row added with `qctl add`.
  Proof: `openspec validate script-snapshots --strict` and
  `mise run q check` pass.
  - 2026-11-08 UTC: both passed, with `mise run check:repo`.
- [x] 2. Pin `insta = "=1.49.0"` with no default features as a
  dev-dependency, `cargo-insta` 1.49.0 in `mise.dev.toml`, and
  `INSTA_UPDATE=no` in `mise.ci.toml`.
  Proof: `Cargo.lock` adds only `insta` and `similar`.
  - 2026-11-08 UTC: `cargo metadata` locked `insta` 1.49.0 and `similar`
    2.7.0, and nothing else.
- [ ] 3. Pin zsh, fish, and shellcheck in `mise.toml` and lock them for
  linux-x64 and macos-arm64.
  Proof: `mise lock` for those tools on both platforms, and
  `mise install --locked` installs them.
  - 2026-11-08 UTC: locked for both platforms; `mise install --locked`
    installed zsh 5.9.2, fish 4.9.3, and shellcheck 0.11.0 on macos-arm64.
    linux-x64 is not yet proven; CI installs it.
- [ ] 4. Add `tests/support/shells.rs`, and move the completion script and
  help tests to snapshots that pass their shell's check.
  Proof: the completion tests pass with `INSTA_UPDATE=no`, locally and in
  CI.
  - 2026-11-08 UTC: on a macOS arm64 workstation, all 11 completion tests
    passed with `INSTA_UPDATE=no`, and clippy on the test crate passed
    with `-D warnings`. The bash, zsh, and fish snapshots were written by
    hand from `clap_complete`'s templates and matched on the first run; the
    help snapshot was rendered by insta. CI has not run yet.
- [ ] 5. State the closed `Shell` set in its rustdoc, and record the
  testing rule in `AGENTS.md`.
  Proof: `mise run verify` passes in CI on the pushed commit.
- [ ] 6. Open the PR ready, wait for CI and Kody, and answer each thread
  in the top PR before the stack merges.
  Proof: CI green on the last push.

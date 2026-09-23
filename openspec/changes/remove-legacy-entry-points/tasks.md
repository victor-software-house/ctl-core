# Tasks

Compile, format, lint, and test tasks run on the build host with `mise run verify`.

## 1. Remove

- [x] 1.1 Confirm no owned consumer calls a wrapper; proof: no call to `go`, `run`, `main`, `main_with`, or `main_with_help` in qctl, verctl, forkctl, or vpn-proxy-kit-rs
- [x] 1.2 Delete `src/run.rs`, its module, the crate-root `run` alias, and `prelude::main_with_help`; proof: `mise run verify` passes
- [x] 1.3 Remove `help::emit_bare`, move `dep:anyhow` from `cli` to `app`, and compile `parser::requires_input` only with `app`; proof: `mise run check:features` reports no new warning
- [x] 1.4 Show `App` in the prelude documentation and drop the `go` wrapper from the migration surfaces in `AGENTS.md`; proof: no `go` remains in either file
- [x] 1.5 Add a patch changeset; proof: `.changeset/remove-legacy-entry-points.md`
- [ ] 1.6 Close CTC-010 when the pull request merges; proof: `mise run q check` passes

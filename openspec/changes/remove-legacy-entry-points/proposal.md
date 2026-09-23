# Proposal

## Why

`App` is the entry point every consumer uses. qctl 0.4.3, verctl 0.3.2,
forkctl 0.0.26, and vpn-proxy-kit-rs all enter through `App::run`. The older
wrappers in `src/run.rs` are still public:

1. `go` parses a Clap type, prints help, and runs a closure.
2. `main`, re-exported at the crate root as `run`, maps a closure result to an
   exit code.
3. `main_with` does the same with an explicit format and colour.
4. `main_with_help`, also in the prelude, prints help and then calls `main`.

No owned consumer calls any of them. They are a second lifecycle beside `App`,
with their own help and error paths, and the prelude documentation still
shows `go` as the way to write `main`. CTC-010 closes only when `go` is gone
and no compatibility alias remains.

Queue row: CTC-010, "Own the unified ctl presentation kernel".

## What Changes

1. Remove `src/run.rs` and every export of it: `run::go`, `run::main`,
   `run::main_with`, `run::main_with_help`, the crate-root `run` alias, and
   `prelude::main_with_help`.
2. Remove what only the wrappers used: `help::emit_bare`, and the `anyhow`
   dependency of the `cli` feature. `anyhow` moves to `app`, the only feature
   whose public API returns its `Result`, and `parser::requires_input` is
   compiled only with `app`.
3. Show `App` in the prelude documentation.
4. Stop calling the `go` wrapper a migration surface in `AGENTS.md`.

## Capabilities

### New Capabilities

- `entry`: how a consumer binary enters ctl-core.

### Modified Capabilities

None.

## Impact

1. A consumer that still calls a removed wrapper stops compiling. No owned
   consumer does.
2. The release is a patch, as every 0.x release is.

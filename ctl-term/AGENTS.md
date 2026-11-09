# ctl-term

Terminal output for CLIs: colour policy per stream, colour depth, width, and the escape encoder.
The repository's root [`AGENTS.md`](../AGENTS.md) holds the workspace-wide rules.
This file holds what applies to this crate alone.

## What it owns

- `Terminal`: what is known about one stream, decided once from a probe, a colour mode, and an environment lookup.
- `ColorMode`, `ColorDepth`, and the nearest-colour conversion between depths.
- `DetectOptions`: the width rules and every default they apply, and the `DEFAULT_*` constants other crates use instead of copying the numbers.
- `Encoder`: the only place an escape sequence is written.
  It needs no terminal, so ctl-core's string rendering calls it with a depth it chose.
- `Console`: renders a `ctl_text::Renderable` at its stream's width and writes it.
  It reports a closed pipe as `Error::BrokenPipe`.

## What it must not own

- Layout, tables, records, or widgets.
  Those belong to `ctl-layout`; text measurement and wrapping belong to `ctl-text`.
- The exit code for a closed pipe.
  This crate reports the error, and the caller chooses what to do.
- A dependency on a terminal crate such as `anstream` or `crossterm`.
  The crate depends on `anstyle` and `ctl-text`, and on `terminal_size` behind the `detect` feature.

## Platforms and features

- The crate supports and tests Unix only.
  It has no Windows path, no virtual-terminal mode, and no `anstyle-query`.
  Windows is to come with a Windows CI job; until then do not add `cfg(windows)` code.
  [Design decision 19][unix] records the reason.
- `detect` is a default feature.
  It holds everything that asks the operating system: `terminal_size`, `Probe::of`, `Terminal::detect`, the `from_process` constructors, and `Console::print`.
  The crate must compile with `--no-default-features`, because ctl-core's `render` tier takes only the `Encoder` from it.
  Gate a new item that touches the process or the operating system on `detect`.

## Public API

- A public item is a promise.
  Every public item has a doc comment that adds something beyond its name.
  The doc gives what the item is for, a unit, an invariant, an edge case, or what it returns.
- The enums are `#[non_exhaustive]`, so a new variant is not a break.
- A change that breaks the public API needs a minor changeset in the repository's `.changeset/` folder.
  CI runs `mise run semver` against the last release.
- Environment reads go through the `env` lookup that the caller passes.
  `Terminal::from_probe` touches no process state, so keep it that way.

## Tests

- `tests/it/detection.rs`: tables over every variable that decides colour, depth, and width, each alone and in conflicting pairs.
- `tests/it/scenarios.rs`: the `terminal-output` spec scenarios this crate owns, each through a `Console`.
- `tests/it/encoding.rs`, `palette.rs`, and `properties.rs`: the encoder, the colour conversion, and proptest properties of escape output.
- `tests/closed_pipe.rs`: a real child process piped to `head -1`.
  It needs the `detect` feature, and on a target that is not Unix it prints a skip line.
  It runs without libtest, because libtest prints its own banner to stdout, and `head -1` quits after the first line.
  The next libtest write would then fail before the code under test ran.
  Keep that target at `harness = false` in `Cargo.toml`.
- `tests/non_utf8_no_color.rs`: a child process started with a `NO_COLOR` that is not valid UTF-8, which a test cannot set in its own environment without `unsafe`.
  The child asserts that `Terminal::from_process` reports no colour, and a control child without `NO_COLOR` asserts that colour is on.
  It needs the `detect` feature, and on a target that is not Unix it prints a skip line.
  It keeps `harness = false` so the child is a plain `main` that reports through its exit code.
- The `it` tests pass an explicit environment lookup and probe (`tests/it/common.rs`), so they read no process variable.

## Running them

`mise run verify` runs them with the rest of the workspace.
To run this crate's tests alone:

```sh
cargo test --locked -p ctl-term --all-features
```

`mise run lint:prose ctl-term` checks the prose of the crate's docs and README without compiling.

## Design decisions

These bind the crate.
They live in [`design.md`](../openspec/changes/terminal-render-engine/design.md).

- [Escape sequences are written once, by the sink][escape]: layout produces lines, and the encoder adds every escape.
- [Terminal facts are a pure function of an environment snapshot][facts]: the colour table and the width order.
- [Colour depth is detected and every colour is downgraded to it][depth].
- [A closed pipe ends the program quietly][pipe]: the crate reports `BrokenPipe`.

[escape]: ../openspec/changes/terminal-render-engine/design.md#5-escape-sequences-are-written-once-by-the-sink
[facts]: ../openspec/changes/terminal-render-engine/design.md#10-terminal-facts-are-a-pure-function-of-an-environment-snapshot
[depth]: ../openspec/changes/terminal-render-engine/design.md#11-colour-depth-is-detected-and-every-colour-is-downgraded-to-it
[unix]: ../openspec/changes/terminal-render-engine/design.md#19-ctl-term-supports-and-tests-unix-only-for-now
[pipe]: ../openspec/changes/terminal-render-engine/design.md#13-a-closed-pipe-ends-the-program-quietly

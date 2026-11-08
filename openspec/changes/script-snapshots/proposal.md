# Reviewed snapshots for rendered scripts and help

## Why

The `completion` verb's tests check its scripts with string literals and
`contains`.
The fish test holds the whole line in an `indoc!` literal.
The bash test checks 4 lines of a 34-line script, and the zsh test checks
its first line, 2 inner lines, and its last 4 lines.
The help test checks 3 substrings.
So most of each script is untested, and a change to any line shows up as a
test edit rather than as a script diff.

No test runs a script through its shell either.
A script that bash, zsh, or fish cannot read would still pass.

The shell-integration change will add three more scripts, each written by
ctl-core.
It needs the same practice from its first test.

Queue row: CTC-031.

## What Changes

1. `insta`, pinned to `=1.49.0` as a dev-dependency with no default
   features, keeps each rendered script and the verb's help as a reviewed
   snapshot under `tests/snapshots/`.
2. CI sets `INSTA_UPDATE=no`, so a changed or missing snapshot fails the
   test and CI never writes one.
   `cargo-insta` 1.49.0, a local tool, reviews changes.
3. A shared test helper runs each script through its own shell:
   - `bash -n` and `shellcheck --enable=all` for bash;
   - `zsh -f -n` for zsh;
   - `fish --no-config -n` for fish.

   It writes the script through ctl-core's `fixture` module.
4. zsh 5.9.2, fish 4.9.3, and shellcheck 0.11.0 are lazy tools in
   `mise.toml`, so local runs and CI use the same tools, and only the
   `test` task, through its `install:test-tools` dependency, fetches them.
5. The `completion` tests move to snapshots.
   The test crate drops its `missing_docs` exception, which never fired.
6. The `Shell` docs state that its set of shells is closed.
7. The testing rule is recorded in `AGENTS.md`.

## Capabilities

### New Capabilities

- `snapshot-tests`: how ctl-core's tests keep rendered output as reviewed
  snapshots, and how they check a rendered shell script.

### Modified Capabilities

None.

## Impact

1. Tests: `tests/completion.rs`, `tests/support/shells.rs`, and
   `tests/snapshots/`.
2. Tools: `mise.toml`, `mise.lock`, `mise.dev.toml`, `mise.dev.lock`, and
   `mise.ci.toml`.
3. `Cargo.toml` and `Cargo.lock` gain `insta` and `similar` as test-only
   crates.
   A consumer's dependency graph does not change.
4. Docs: `AGENTS.md` and the rustdoc of `Shell`.

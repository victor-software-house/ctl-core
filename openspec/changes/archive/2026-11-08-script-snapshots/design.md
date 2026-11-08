# Design

## Context

ctl-core had no snapshot library.
The tests that compare a rendered operator document with its committed file
read the file and compare bytes, with no tool to review or accept a change.
Four ways to keep expected output were read from source at the versions
below.

1. [insta] 1.49.0 stores each snapshot as a `.snap` file: a short YAML
   header, then the value as it is.
   `cargo insta review` shows each change as a line diff and accepts or
   rejects it.
   `INSTA_UPDATE` picks whether a failing test writes anything.
   - `no` never writes.
   - The default, `auto`, writes `.snap.new` files for review, or behaves
     as `no` when it detects CI ([env.rs][insta-env],
     [utils.rs][insta-ci]).

   With default features off, it adds `similar` and two crates ctl-core
   already locks, `once_cell` and `tempfile`.
2. [snapbox] 1.2.2 compares against a plain file named by `file![…]`, and
   `SNAPSHOTS=overwrite` rewrites it ([action.rs][snapbox-action]).
   It has no review tool: an overwrite lands in the working tree, and
   `git diff` is the review.
   Its default features add colour and diff crates.
3. [expect-test] 1.5.1 keeps the expected value in the test source, or in a
   file named by `expect_file!`, and `UPDATE_EXPECT=1` rewrites it in place
   ([lib.rs][expect-lib]).
   It has no review step and no CI guard.
   Its last release was 2024-12-21.
4. Plain files under `tests/` compared by a helper of our own.
   This needs no dependency, but ctl-core would write and maintain the
   update and review steps itself.

## Decisions

1. **insta, pinned to `=1.49.0`, as a dev-dependency.**
   Each rendered script and help page is an `assert_snapshot!` on a named
   file snapshot, `tests/snapshots/<test file>__<name>.snap`.
   The diff of a change to rendered output is the diff of that file.
   - Alternative: snapbox.
     Its plain files would let a shell read a snapshot directly, but it
     has no review step, and an overwrite is silent until `git diff`.
     The test already holds the rendered text, so it can give that text
     to the shell; a plain file adds nothing there.
   - Alternative: expect-test.
     Inline expectations are the long literals this change removes.
     Its file form has no review step, and the crate has had no release
     since 2024.
   - Alternative: plain files and our own helper.
     That would be a second implementation of what insta does, owned here.

   The rust-projects guide already names insta for output formats.
   The exact pin keeps the snapshot file format fixed until a reviewed
   bump.
   Default features are off, so no colour crate is added and CI logs stay
   plain.
2. **CI never writes a snapshot.**
   `mise.ci.toml` sets `INSTA_UPDATE=no`.
   insta would already pick `no` when `CI` is set, but setting it
   explicitly does not depend on how the runner identifies itself.
   A developer overrides it locally with `INSTA_UPDATE=always`, or reviews
   with `cargo insta review` from `cargo-insta` 1.49.0 in `mise.dev.toml`.
   CI never loads that file.
3. **Each script passes its own shell's syntax check.**
   `tests/support/shells.rs` writes the script through
   `fixture::Suite`, then runs:
   - `bash -n` and `shellcheck --shell=bash --enable=all` for bash;
   - `zsh -f -n` for zsh;
   - `fish --no-config -n` for fish.

   A non-zero exit fails the test with the tool's output.
   - Alternative: parse the snapshot files in a separate mise task.
     A `.snap` header is not shell syntax, and the test already holds the
     text the snapshot compares.
4. **The tools are a `test` profile**: `conda:zsh` 5.9.2, `conda:fish`
   4.9.3, and `shellcheck` 0.11.0 in `mise.test.toml`, locked in
   `mise.test.lock`.
   `.miserc.toml` loads `test` after `dev`, so local runs use the same
   tools.
   In CI, the shared workflow loads it only in its Build and test job, as
   `MISE_ENV=ci,test`, through its `test-profiles` input
   ([actions#55][actions-55]); the repository template renders both settings
   from its `test_profiles` answer.
   The lint, docs, and release jobs never fetch the tools.
   A profile's tools come from a config file, so mise looks up their conda
   dependency packages in that file's own lockfile, `mise.test.lock`
   ([lockfile.rs L2606-2608][mise-config-path],
   [conda.rs L552-569][mise-conda-deps]).
   Their install directories are on the tests' `PATH`: mr-boxington's
   command wrapper takes only mise's shims off it
   ([exec.rs L387-391][mise-wrapper-path]).
   Conda-forge builds cover both locked platforms, linux-x64 and
   macos-arm64; fish's GitHub release has no macOS archive.
   bash stays the platform's own shell.
   A test that cannot start a tool fails and names it.
   - Alternative: skip when a tool is missing.
     That check would pass without checking anything.
   - Alternative: eager `[tools]` in `mise.toml`.
     Every job and every `mise install` would fetch conda zsh, conda fish,
     and their conda dependencies, release jobs included.
   - Alternative: `tools` of a `[tasks.test]` that replaces the catalog's.
     It shipped first, and a cold install failed in CI ([run
     42433739166][run-cold]):
     `conda package pcre2-10.49-h8b3dc9c_0 not found in lockfile for
     linux-x64`, although `mise.lock` holds that package.
     mise 2026.10.25 gives a task tool the source `Argument`
     ([task/mod.rs][mise-task-arg]).
     It finds the tool's version by merging every lockfile
     ([lockfile.rs L5506-5527][mise-locked-version]), but the conda backend
     reads the dependency packages from the lockfile of the request's own
     source ([conda.rs L552-569][mise-conda-deps]), and `Argument` maps to
     none ([lockfile.rs L2648-2650][mise-source-path]), so it reads an
     empty lockfile.
     Earlier runs passed only because their mise cache already held both
     shells.
     mise 2026.11.4 reads the same way.
   - Alternative: lazy tools in `mise.toml`, which a test's first start
     installs through its bootstrap shim ([lazy tools][mise-lazy]).
     cargo runs through mr-boxington's command wrapper, and mise takes its
     shims off a wrapped command's `PATH`
     ([exec.rs L387-391][mise-wrapper-path]), so no test finds the shim
     ([run 42434896632][run-wrapped]).
   - Alternative: lazy tools plus an `install:test-tools` task that `test`
     depends on.
     mise builds a task's toolset when the task starts, after its
     dependencies ([task_executor.rs L2716-2737][mise-task-context]), so the
     install directories reached the tests.
     It was rejected on review as a workaround: a second install step beside
     mise's own, and a local copy of the catalog's `test` task.
   - Alternative: pin the tools in `mise.ci.toml` only.
     Local runs would then use other versions, or none.

   Known mise limit: `mise lock` also writes the pinned catalogs' task tools
   into `mise.test.lock`, beside the test tools.
   mise makes one target per lockfile and gives each only its own config
   files ([lock.rs L2432-2471][mise-lock-targets]).
   A task from a `git::` include has no config file of its own
   ([task/mod.rs L2564-2571][mise-task-cf]), so mise assigns its tools to
   any config file of the target with the task's config root
   ([lock.rs L3050-3088][mise-task-config-path]), and every profile's file
   has that root.
   So every lockfile of a loaded profile holds every catalog task tool, and
   a `mise.test.lock` with only the test tools is reported stale.
   mise 2026.11.4 still works this way.
   The locks are therefore generated with the default profiles,
   `mise lock --platform linux-x64,macos-arm64`, and the pre-push lock check
   runs with `MISE_ENV=dev,test`, the list `.miserc.toml` loads.
5. **shellcheck exclusions are named per caller.**
   The bash script `clap_complete` writes breaks SC2207, SC2181, and
   SC2250, in its own template ([shells.rs][clap-bash]).
   The completion test excludes those three, with that reason.
   A script ctl-core writes itself passes with none.
6. **The `Shell` set is closed.**
   `Shell` is not `#[non_exhaustive]`.
   Each verb that takes it must have a form for every shell, and an
   exhaustive match in a consumer shows the compiler each place a new shell
   needs one.
   Adding a shell is therefore a breaking release, as its rustdoc says.
   - Alternative: `#[non_exhaustive]`.
     A consumer would need a wildcard arm, which hides an unhandled shell at
     run time.
7. **A test crate with no exceptions carries no lint attribute.**
   The `completion` test crate's `#![allow(missing_docs, …)]` became
   `#![expect]`, and the compiler then reported the expectation as
   unfulfilled: `missing_docs` checks exported items, and a test crate
   exports none.
   The attribute is removed.

[clap-bash]: https://github.com/clap-rs/clap/blob/clap_complete-v4.6.11/clap_complete/src/env/shells.rs#L30-L63
[expect-lib]: https://github.com/rust-analyzer/expect-test/blob/v1.5.1/src/lib.rs#L20-L40
[expect-test]: https://github.com/rust-analyzer/expect-test/tree/v1.5.1
[insta]: https://github.com/mitsuhiko/insta/tree/1.49.0
[insta-ci]: https://github.com/mitsuhiko/insta/blob/1.49.0/insta/src/utils.rs#L9-L16
[insta-env]: https://github.com/mitsuhiko/insta/blob/1.49.0/insta/src/env.rs#L245-L269
[actions-55]: https://github.com/victor-software-house/actions/pull/55
[mise-conda-deps]: https://github.com/jdx/mise/blob/v2026.10.25/src/backend/conda.rs#L552-L569
[mise-config-path]: https://github.com/jdx/mise/blob/v2026.10.25/src/lockfile.rs#L2606-L2608
[mise-lazy]: https://github.com/jdx/mise/blob/v2026.10.25/docs/dev-tools/shims.md#L435-L451
[mise-lock-targets]: https://github.com/jdx/mise/blob/v2026.11.4/src/cli/lock.rs#L2432-L2471
[mise-locked-version]: https://github.com/jdx/mise/blob/v2026.10.25/src/lockfile.rs#L5506-L5527
[mise-source-path]: https://github.com/jdx/mise/blob/v2026.10.25/src/lockfile.rs#L2648-L2650
[mise-task-arg]: https://github.com/jdx/mise/blob/v2026.10.25/src/task/mod.rs#L168
[mise-task-cf]: https://github.com/jdx/mise/blob/v2026.11.4/src/task/mod.rs#L2564-L2571
[mise-task-config-path]: https://github.com/jdx/mise/blob/v2026.11.4/src/cli/lock.rs#L3050-L3088
[mise-task-context]: https://github.com/jdx/mise/blob/v2026.10.25/src/task/task_executor.rs#L2716-L2737
[mise-wrapper-path]: https://github.com/jdx/mise/blob/v2026.10.25/src/cli/exec.rs#L387-L391
[run-cold]: https://github.com/victor-software-house/ctl-core/actions/runs/42433739166
[run-wrapped]: https://github.com/victor-software-house/ctl-core/actions/runs/42434896632
[snapbox]: https://github.com/assert-rs/snapbox/tree/snapbox-v1.2.2
[snapbox-action]: https://github.com/assert-rs/snapbox/blob/snapbox-v1.2.2/crates/snapbox/src/assert/action.rs#L1-L33

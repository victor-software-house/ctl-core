# Tasks

## 1. Open types

- [x] 1.1 Mark `Block`, `OutputFormat`, and `NoticeLevel` `#[non_exhaustive]`, each with a sentence on the wildcard arm.
  Verify: `mise run verify` passes on the build host.
- [x] 1.2 Mark `ErrorBody`, `FlagWarning`, `Surface`, `SurfaceCommand`, `SurfaceArgument`, and both `Envelope` variants `#[non_exhaustive]`.
  Verify: `mise run verify` passes, and no struct literal of them remains outside `ctl-core/src/`.
- [x] 1.3 Mark `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`, `DryRunArgs`, `CompletionArgs`, and `InitArgs` `#[non_exhaustive]`.
  Keep `CompletionCommand` and `InitCommand` exhaustive, and make `tests/completion.rs` and `tests/shell_init.rs` match the arguments without building them.
  Verify: both test targets compile and their parse tests pass.

## 2. Fields with one reader

- [x] 2.1 Make `color` and `no_color` private in `OutputArgs` and `ColorLong`.
  Verify: `tests/cli/flags_cli.rs` reads the colour mode only through `color()`, and passes.
- [x] 2.2 Make `View`'s `format` and `quiet` private, and add `View::format()` and `View::is_quiet()`.
  Verify: `app.rs` reads them through the methods, and the `app` unit tests pass.

## 3. Errors keep their I/O failure

- [x] 3.1 Give `InputError` and `ServeError` a private `io_failure`, read through `io_failure()`, with `#[non_exhaustive]` on both.
  Build every value through a private constructor.
  Verify: both still derive `Clone`, `PartialEq`, and `Eq`, and `mise run verify` passes.

## 4. Sealed trait and closed sets

- [x] 4.1 Seal `EnvPrefix` with a private supertrait, at its public path and in the prelude.
  Verify: `tests/env_flags.rs` passes, and the prelude still exports `EnvPrefix`.

  cargo-semver-checks 0.51.0 against [c0c7ac3][c0c7ac3] does not report the seal: neither `trait_newly_sealed` nor `trait_added_supertrait` fires.
  Not verified: why it does not, and that a crate outside ctl-core fails to implement the trait.
- [x] 4.2 Say in one sentence why each closed enum is closed: `Outcome`, `Destination`, `BrokenPipe`, `ColorMode`, `SurfaceRequirement`, `SurfaceScope`, and `FlagClash`.
  Verify: each rustdoc holds the sentence, and `doc`, `doc:default-features`, and `doc:no-default-features` pass.

## 5. Docs and release

- [x] 5.1 State the rules under Public API in `ctl-core/AGENTS.md`.
  Verify: `mise run lint:prose` passes.
- [x] 5.2 Write one `ctl-core: minor` changeset that lists each break and its migration.
  Verify: cargo-semver-checks against the previous main reports only breaks the changeset lists.

## Evidence

2026-11-09 (-03:00), on the build host in a fresh Linux microVM (8 vCPUs, 16 GiB, NixOS, bash 5.3.20), at [565b916][565b916], with `MISE_ENV=ci,test`:

1. `mise run --continue-on-error verify` ran every task.
   All passed except two, and both fail the same way on main at [c0c7ac3][c0c7ac3] in the same machine:
   - `deny`: 23 `unused-workspace-dependency` errors from cargo-deny.
   - `test`: 3 bash tests in `shell_init` (`bash_runs_the_hook_once_from_a_string`, `_from_an_array`, `_after_bash_preexec_rewrites_the_prompt_command`).
     The other 11 `shell_init` tests and every other test target passed, `completion` and `env_flags` included.
2. `check:versions` passed once the clone had an `origin/main`, and `lint:prose`, `specs`, and `doc`, `doc:default-features`, and `doc:no-default-features` passed with no warning.
3. `cargo semver-checks check-release -p ctl-core --baseline-rev c0c7ac3 --all-features` (0.51.0) reported 5 major checks.
   Each item it names is in the changeset:
   - `enum_marked_non_exhaustive`: `Block`, `NoticeLevel`, `OutputFormat`.
   - `enum_variant_marked_non_exhaustive`: `Envelope::Ok`, `Envelope::Err`.
   - `struct_marked_non_exhaustive`: `ColorLong`, `DryRunArgs`, `FormatArgs`, `FormatLong`, `InitArgs`, `Surface`, `SurfaceArgument`, `SurfaceCommand`.
   - `struct_pub_field_missing` and `struct_pub_field_now_doc_hidden`: `OutputArgs.color`, `OutputArgs.no_color`, `ColorLong.color`, `ColorLong.no_color`, `View.format`, `View.quiet`.

   It did not report `ErrorBody`, `FlagWarning`, `CompletionArgs`, `InputError`, `ServeError`, or `OutputArgs` as newly `#[non_exhaustive]`, nor the seal on `EnvPrefix`, though each changed as the changeset says.
   `mise run semver` skips the check while a minor fragment is pending.
4. Not verified here: CI on the pull request.

[c0c7ac3]: https://github.com/victor-software-house/ctl-core/commit/c0c7ac36e3d15c96726c9ea9a50b04a771d99a66
[565b916]: https://github.com/victor-software-house/ctl-core/commit/565b916f920abc3f004fce25a70237681c6be0fe

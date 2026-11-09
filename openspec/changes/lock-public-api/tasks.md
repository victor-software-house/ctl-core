# Tasks

## 1. Open types

- [ ] 1.1 Mark `Block`, `OutputFormat`, and `NoticeLevel` `#[non_exhaustive]`, each with a sentence on the wildcard arm.
  Verify: `mise run verify` passes on the build host.
- [ ] 1.2 Mark `ErrorBody`, `FlagWarning`, `Surface`, `SurfaceCommand`, `SurfaceArgument`, and both `Envelope` variants `#[non_exhaustive]`.
  Verify: `mise run verify` passes, and no struct literal of them remains outside `ctl-core/src/`.
- [ ] 1.3 Mark `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`, `DryRunArgs`, `CompletionArgs`, and `InitArgs` `#[non_exhaustive]`.
  Keep `CompletionCommand` and `InitCommand` exhaustive, and make `tests/completion.rs` and `tests/shell_init.rs` match the arguments without building them.
  Verify: both test targets compile and their parse tests pass.

## 2. Fields with one reader

- [ ] 2.1 Make `color` and `no_color` private in `OutputArgs` and `ColorLong`.
  Verify: `tests/cli/flags_cli.rs` reads the colour mode only through `color()`, and passes.
- [ ] 2.2 Make `View`'s `format` and `quiet` private, and add `View::format()` and `View::is_quiet()`.
  Verify: `app.rs` reads them through the methods, and the `app` unit tests pass.

## 3. Errors keep their I/O failure

- [ ] 3.1 Give `InputError` and `ServeError` a private `io_failure`, read through `io_failure()`, with `#[non_exhaustive]` on both.
  Build every value through a private constructor.
  Verify: both still derive `Clone`, `PartialEq`, and `Eq`, and `mise run verify` passes.

## 4. Sealed trait and closed sets

- [ ] 4.1 Seal `EnvPrefix` with a private supertrait, at its public path and in the prelude.
  Verify: `tests/env_flags.rs` passes, and cargo-semver-checks against the previous main reports `trait_newly_sealed` for it.
- [ ] 4.2 Say in one sentence why each closed enum is closed: `Outcome`, `Destination`, `BrokenPipe`, `ColorMode`, `SurfaceRequirement`, `SurfaceScope`, and `FlagClash`.
  Verify: each rustdoc holds the sentence, and `doc`, `doc:default-features`, and `doc:no-default-features` pass.

## 5. Docs and release

- [ ] 5.1 State the rules under Public API in `ctl-core/AGENTS.md`.
  Verify: `mise run lint:prose` passes.
- [ ] 5.2 Write one `ctl-core: minor` changeset that lists each break and its migration.
  Verify: cargo-semver-checks against the previous main reports only breaks the changeset lists.

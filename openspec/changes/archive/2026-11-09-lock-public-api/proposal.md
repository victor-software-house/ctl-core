# Lock the public API before 0.7.0

## Why

0.7.0 is ctl-core's one minor release in this cycle.
After it, every change below breaks a consumer, and so needs another minor release:

1. A new `Block`, such as a panel from [CTC-035][ledger], breaks every match on `Block` outside ctl-core.
2. A YAML output format, which [CTC-043][ledger] implies, breaks every match on `OutputFormat`.
3. A new field on a public-field struct breaks every struct literal of it outside ctl-core.
   That covers the envelope, the flag structs, the warnings, and the surface types.
4. A new method on `EnvPrefix` breaks any type outside ctl-core that implements it.

Two more defects exist today:

1. `InputError` and `ServeError` turn an `io::Error` into a string.
   A caller cannot tell a missing file from a refused one without parsing the message.
2. `OutputArgs` and `ColorLong` expose `color` and `no_color` as public fields.
   A caller that reads `color` directly skips `--no-color`.
   `View` exposes `format` and `quiet` as public fields, where every other setting is a method.

This change belongs to [CTC-045][ledger].

## What Changes

1. **BREAKING** `#[non_exhaustive]` on these enums: `document::Block`, `format::OutputFormat`, and `document::NoticeLevel`.
2. **BREAKING** `#[non_exhaustive]` on these structs: `model::ErrorBody`, `flags::FlagWarning`, and `Surface`, `SurfaceCommand`, and `SurfaceArgument`.
3. **BREAKING** `#[non_exhaustive]` on both `Envelope` variants. `Envelope::ok`, `Envelope::err`, and `ErrorBody::new` build them.
4. **BREAKING** `#[non_exhaustive]` on the flattened clap `Args` structs:
   `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`, `DryRunArgs`, `CompletionArgs`, and `InitArgs`.
   `CompletionCommand` and `InitCommand` stay exhaustive.
5. **BREAKING** `color` and `no_color` become private in `OutputArgs` and `ColorLong`. `color()` is their only reader.
6. **BREAKING** `InputError` and `ServeError` become `#[non_exhaustive]`.
   Each keeps the `io::ErrorKind` of an I/O failure in a private field, read through `io_failure()`.
7. **BREAKING** `View`'s `format` and `quiet` become private.
   `View::format()` and `View::is_quiet()` read them, and the `quiet(bool)` setter stays.
8. **BREAKING** `EnvPrefix` is sealed by a private supertrait, at its public path.
9. Each enum that stays exhaustive says in its rustdoc why its set is closed.
   These are `Outcome`, `Destination`, `BrokenPipe`, `ColorMode`, `SurfaceRequirement`, `SurfaceScope`, and `FlagClash`.

Nothing changes in `ctl-text`, `ctl-layout`, or `ctl-term`.

## Capabilities

### New Capabilities

- `public-api`: which ctl-core types may grow in a minor release, which sets are closed, and what an error keeps of an I/O failure.

### Modified Capabilities

None.

## Impact

1. Code: `ctl-core/src/` `document.rs`, `format.rs`, `model.rs`, `input.rs` and `input/`, `served.rs`, `flags/`, `completion.rs`, `shell_init.rs`, `view.rs`, `app.rs`, `color.rs`, and `surface.rs`.
2. Tests: `ctl-core/tests/completion.rs` and `ctl-core/tests/shell_init.rs` match the verbs' arguments without building them.
3. Docs: `ctl-core/AGENTS.md`, under Public API, and the rustdoc of every changed type.
4. Consumers:
   - A consumer that builds one of these types with a struct literal changes that code.
     So does one that matches an open enum without a wildcard arm.
   - A consumer that reads `OutputArgs::color` calls `color()`.
   - A consumer that reads `View::format` or `View::quiet` calls `format()` or `is_quiet()`.
   - A consumer that reads `InputError::problems` for an I/O failure can call `io_failure()` instead.
5. Release: one `ctl-core: minor` changeset lists each break and its migration.

[ledger]: ../../../../tasks.yaml

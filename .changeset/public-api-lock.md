---
ctl-core: minor
"verctl:group": changed
---

Lock the public API so that later releases can add variants, fields, and trait methods without breaking a consumer.

Each break and its migration:

- `document::Block`, `format::OutputFormat`, and `document::NoticeLevel` are `#[non_exhaustive]`.
  A match on one of them outside ctl-core ends with a `_` arm.
- `model::ErrorBody` and both `Envelope` variants are `#[non_exhaustive]`.
  Build them with `ErrorBody::new`, `Envelope::ok`, and `Envelope::err`, and end a pattern on a variant with `..`.
- `flags::FlagWarning`, `Surface`, `SurfaceCommand`, and `SurfaceArgument` are `#[non_exhaustive]`.
  Read their fields as before. A struct literal of one of them no longer compiles.
- `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`, `DryRunArgs`, `CompletionArgs`, and `InitArgs` are `#[non_exhaustive]`.
  Get them from the parse, and end a pattern on `CompletionArgs` or `InitArgs` with `..`.
  `CompletionCommand` and `InitCommand` are unchanged.
- `OutputArgs` and `ColorLong` no longer expose `color` and `no_color`.
  Call `color()`, which applies `--no-color`.
- `View`'s `format` and `quiet` fields are private.
  Read them with `View::format()` and `View::is_quiet()`. The `quiet(bool)` setter is unchanged.
- `InputError` and `ServeError` are `#[non_exhaustive]`.
  Their public fields stay, and `io_kind()` returns the `io::ErrorKind` when a file could not be read or written.
- `EnvPrefix` is sealed. `#[command(env_prefix = "...")]` works as before, and no type outside ctl-core can implement the trait.

`Outcome`, `Destination`, `BrokenPipe`, `ColorMode`, `SurfaceRequirement`, `SurfaceScope`, `FlagClash`, and `Shell` stay exhaustive.
Each one's docs say why its set is closed.

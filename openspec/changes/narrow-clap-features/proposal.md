# Narrow Clap features and a shared verbatim predicate

## Why

A ctl CLI pays for every dependency feature ctl-core turns on, and a
consumer cannot turn a feature off once ctl-core asks for it. ctl-core 0.6.28
declares `clap` without `default-features = false`, so every consumer
compiles Clap's `color`, `help`, `usage`, `error-context`, and
`suggestions`, whatever its own `clap` entry says.

ctl-core needs none of them for `cli`. It renders help itself, through its
own document renderer, and prints parse errors through `Display`, which is
plain text. The only calls that needed a Clap feature were four
`Command::color` calls, which need `color`, and `Error::print`, which
prints Clap's own help when a command misses a required subcommand.

Measured on a toy CLI at the `app` tier, with each feature removed from a
copy of 0.6.28 in turn (opt-level `"s"`, fat LTO, one codegen unit, abort,
strip; darwin-arm64; `__text` bytes): Clap's `help` costs 30.1 KiB, its
`error-context` 10.1 KiB, `suggestions` 6.2 KiB, and `color` 0.9 KiB.
[Measurements](design.md#measurements) has the full table.

The same audit found that the renderer's verbatim filter called
`unicode_general_category::get_general_category` to test two characters,
U+2028 and U+2029, the only members of categories Zl and Zp. That call links
a lookup table for all of Unicode: 49,536 bytes of `__const` in a two-binary
test. The bidi controls beside it come from `unicode_bidi::format_chars`,
which are plain constants and cost nothing; the same two-binary test
measured equal file sizes. A CLI that prints a title it did not write needs
the same predicate, and today keeps its own copy.

Queue row: CTC-032.

## What Changes

1. `clap` is declared with `default-features = false`, and `cli` turns on
   only `std`, `derive`, `env`, and `string`. `help` adds `clap/usage` for
   the usage line it renders. Error context, suggestions, color, and Clap's
   own help stay in the consumer's own `clap` entry.
2. `App` prints help from ctl-core's renderer whenever Clap stops for help:
   on `-h` and `--help` as before, and now also when a command that requires
   an argument or subcommand gets neither, which goes to stderr with exit
   code 2. The version prints Clap's plain version line. ctl-core no longer
   calls `Error::print` or `Command::color`.
3. A new `verbatim` feature and public module, `ctl_core::verbatim`, owns
   which characters are unsafe to print verbatim: `is_unsafe(char)`, with
   `LINE_SEPARATOR` and `PARAGRAPH_SEPARATOR` beside it and the bidi controls
   taken from `unicode_bidi::format_chars`. `render` enables it, and
   `Document::verbatim` filters through it.
4. `unicode-general-category` leaves the dependency graph.

## Impact

1. A consumer that wants Clap's error context or suggestions turns them on
   in its own `clap` entry; features unify, so nothing else changes for it.
   A consumer that leaves its `clap` entry on the default features keeps
   every Clap feature and sees no size change.
2. A consumer that turns on ctl-core `usage` still compiles every Clap
   default feature: `usage-lib` 6 declares `clap` without
   `default-features = false`.
3. A command that misses a required subcommand prints ctl-core's styled
   help instead of Clap's.
4. The private table engine stays Comfy Table. Replacing it is a separate
   decision ([design](design.md#decisions), decision 4).

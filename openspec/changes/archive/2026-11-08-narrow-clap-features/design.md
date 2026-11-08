# Design

## Context

Cargo features unify across a build: a feature any crate in the graph turns
on is on for everyone. ctl-core is a library under every ctl CLI, so each
feature it asks for is a floor its consumers cannot go below. The rule this
change follows is that ctl-core asks only for what its own code calls, and
leaves the rest to the consumer.

## Decisions

1. **`cli` asks Clap for `std`, `derive`, `env`, and `string` only.**
   `derive` builds the shared `Args` types, `env` and `string` carry the
   `<CLI>_<LONG>` variable binding, and `help` adds `usage` for the usage
   line `help::document` renders.
   Alternative: keep Clap's default features. It lost because it forces
   about 47 KiB of `__text` on a consumer that turns them off in its own
   `clap` entry.
   Alternative: forward Clap's features as ctl-core features, such as
   `ctl-core/suggestions`. It lost because a consumer already names its own
   `clap` dependency, and a second name for the same switch couples
   ctl-core's feature list to Clap's.
2. **ctl-core prints every help Clap stops for.** `App` already rendered
   `-h` and `--help` through its own document. A command that misses a
   required argument or subcommand still went through `Error::print`, which
   prints Clap's own help and needs Clap's `help` feature for any text at
   all. It now renders the selected command's help document to stderr, and
   the version prints `Error`'s plain `Display`. The `Command::color` calls,
   which need Clap's `color` feature, go with it: `Display` never carries
   ANSI, so no remaining path needs them.
   Alternative: keep `Error::print` and require Clap's `help` from `app`.
   It lost on size, 30.1 KiB of `__text`, and because it printed a second
   help style beside ctl-core's.
3. **One public predicate for verbatim text.** `verbatim::is_unsafe` holds
   the set, and the renderer and any CLI that prints outside text call it.
   The set is three whole sets, each by its definition and never only the
   members a caller needed: general category Cc through `char::is_control`;
   all 12 directional formatting characters of UAX #9 from
   `unicode_bidi::format_chars`; and Zl and Zp, one character each, as named
   constants beside the predicate. unicode-bidi 0.3.18 targets Unicode
   16.0.0, and its 12 constants match the 12 characters [UAX #9 revision
   50][uax9] names. A test pins `unicode_bidi::UNICODE_VERSION`, so a bump
   brings this check back.
   The other format characters (Cf) stay out. They neither act on the
   terminal nor reorder text, and text needs them: ZWJ joins emoji, ZWNJ
   shapes Persian and Indic scripts, the tag characters spell subdivision
   flags, the soft hyphen marks a break, and the Arabic number signs render
   as visible marks. Taking the whole category would break that text, and
   taking only some members would be a set by need, not by definition.
   Alternative: keep `unicode_general_category::get_general_category`. It
   lost because it links a table for all of Unicode, 49,536 bytes, to test
   two characters.
   Alternative: private constants in `render.rs`. It lost because the
   characters already have names in `unicode-bidi`, and because a CLI that
   filters a title would keep a second copy of the set.
4. **The table engine stays Comfy Table for now.** Comfy Table's
   `custom_styling` feature, which ctl-core needs because it paints ANSI
   before layout, turns on `tty` and with it `crossterm`, so Comfy Table
   has no smaller configuration. A private engine of ctl-core's own measured
   a further 150,880 bytes of file and 57,380 bytes of `__text` at the `app`
   tier and drops 22 packages from `Cargo.lock`. It also reverses the rule
   in [presentation] against writing table sizing in-house, so it waits for
   the maintainer's decision and is not part of this change.

No automatic behaviour changes, so there is no new override or disable
path.

## Measurements

A toy CLI with `OutputArgs`, one subcommand, a `Fields` record, and a
`Table`, built at each tier with `default-features = false` on ctl-core and
`clap = { default-features = false, features = ["std", "derive"] }`.
Profile: opt-level `"s"`, `lto = true`, `codegen-units = 1`,
`panic = "abort"`, `strip = true`. rustc 1.99.1, darwin-arm64. `nm` lists
only the dynamic imports, so each binary was stripped. Mach-O pads each
segment to 16 KiB, so the file size moves in 16 KiB steps and `__text` is
the finer measure.

Each Clap feature removed in turn from a copy of 0.6.28, `app` tier,
`__text` bytes:

| Clap features ctl-core asks for | `__text` | Change |
|:--|--:|--:|
| all defaults (0.6.28) | 498,948 | |
| without `color` | 498,052 | −896 |
| also without `suggestions` | 491,856 | −6,196 |
| also without `error-context` | 481,744 | −10,112 |
| `help` removed instead of `error-context` | 461,584 | −30,272 |

The two-binary test of `unicode-bidi`'s constants against the same
characters written as literals: both 286,080 bytes of file, `__text`
190,452 and 190,388.

The tier sizes of this change, measured on the commit CI verified, are in
[tasks](tasks.md), task 5.

[presentation]: ../../../../docs/presentation.md
[uax9]: https://www.unicode.org/reports/tr9/tr9-50.html#Directional_Formatting_Characters

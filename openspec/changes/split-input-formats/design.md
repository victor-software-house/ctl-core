# Design

## Context

`input` parses YAML with serde-saphyr, places problems with yamled, and
validates with garde. `input-toml` adds the `toml` crate on top of `input`.
`Limits` overrides serde-saphyr's `Budget` field by field, through four
tiers: the call, the repository's `.ctl/input.yaml`, the process, and
serde-saphyr's default ([`limits.rs`][limits]).
The TOML path reads three of those fields from a resolved `Budget`
([`toml_text.rs`][toml-text]).

qctl and forkctl enable `app`, `input`, and `usage`; verctl enables `input`,
`surface`, and `usage`. They call `Input::new`, `Input::read`,
`Input::parse`, `Input::check`, and `use_repo_limits`. None calls
`Limits::budget`.

## Goals

1. With default features off and only `input-toml` on, ctl-core links none
   of serde-saphyr, yamled, or garde.
2. A consumer that enables `input` compiles and behaves as before.
3. A TOML text meets byte, depth, and node limits with defaults ctl-core
   states, through the call and process tiers.

## Non-goals

1. A TOML repository limits file. The repository tier stays YAML.
2. A change to the serde-saphyr fork. The split needs nothing new from
   `Budget` or its breach type. Moving the pin to the fork's next release is
   separate work.

## Decisions

### 1. One feature per format, and `input` stays YAML with validation

`input-yaml` carries serde, serde-saphyr, and yamled. `input-toml` carries
serde and the `toml` crate. `input` becomes `input-yaml` plus `validate`.
The `input` module compiles when either format feature is on.

The alternative was to make `input` the format-free base and ask every YAML
consumer to add `input-yaml`. It lost because qctl, verctl, and forkctl would
each need a manifest change, and acceptance needs them to build unchanged.

A public base feature with no format was also considered. It lost because an
`Input` that can parse no format is of no use, and `cfg(any(...))` gives the
module without a feature name to maintain.

### 2. Validation needs `validate`

`Input::check`, `Input::load`, and the field placing behind them compile only
with `validate`. A TOML consumer that validates enables `input-toml` and
`validate`, and pays for garde only then.

The alternative was a separate `input-validate` feature. It lost because
`validate` already names garde, and a second name for the same crate would
drift.

### 3. ctl-core owns `Limits` and three defaults

`Limits` keeps its name, its field names, and its field types.
`max_reader_input_bytes`, `max_depth`, and `max_nodes` exist in every build.
The other 17 fields exist only with `input-yaml`.
`Limits` states the three defaults as constants:

| Constant | Value |
|:--|--:|
| `DEFAULT_MAX_READER_INPUT_BYTES` | 268,435,456 |
| `DEFAULT_MAX_DEPTH` | 64 |
| `DEFAULT_MAX_NODES` | 250,000 |

These are serde-saphyr's defaults in vsh-serde-saphyr 0.0.2, the pinned
release. A YAML text meets the same limits as before. A unit test under
`input-yaml` fails when a serde-saphyr update moves its defaults away from
these constants.

The YAML parse builds its `Budget` from serde-saphyr's defaults, then applies
the three constants and every set field. `Limits::budget` becomes private,
since it returns a serde-saphyr type that a TOML-only build cannot name.

Three alternatives lost:

1. Keeping `Budget` as the TOML path's source. Acceptance excludes
   serde-saphyr from a TOML build.
2. Moving `Budget` into a small crate of the serde-saphyr fork. It needs a
   fork release, and it hands a TOML reader a YAML parser's settings type.
3. A separate `TomlLimits` type. It splits the tiers in two, and a consumer
   that reads both formats would set each limit twice.

Gating fields on a feature is additive: a build with `input-yaml` has more
fields. A consumer that writes a `Limits` literal therefore writes
`..Limits::NONE`, as the existing scenarios do. `Limits` stays exhaustive,
because `#[non_exhaustive]` would forbid that form outside ctl-core.

### 4. YAML constructors need `input-yaml`, and one constructor takes a format

`Format::Yaml` exists only with `input-yaml`, and `Format::Toml` only with
`input-toml`. `Input::new`, `Input::read`, and `Input::frontmatter` exist
only with `input-yaml`, since each is YAML by its contract. A new
`Input::new_with_format(name, text, format)` builds an input of any enabled
format from text in memory. `Input::open` works in every build and refuses an
extension whose feature is off, naming that feature.

The alternative was for `Input::new` to take the only enabled format. It
lost because Cargo unifies features across a build. Another crate that turns
on `input-yaml` would switch a TOML crate's text to YAML, with no change in
that crate.

### 5. The repository tier needs `input-yaml`

`REPO_FILE`, `use_repo_limits`, and `Loosened` exist only with `input-yaml`.
A build with `input-toml` alone decides each limit from the call, then the
process, then ctl-core's default. Its limit message reads
`exceeds max_depth (4); the tool sets this limit`, because the user has no
file to raise it in. A build with both formats applies the repository tier to
TOML texts too, and keeps the `raise limits.<field> in .ctl/input.yaml` text.

`use_repo_limits` reports a loosened shared field against ctl-core's
constant, and a YAML-only field against serde-saphyr's default.

The alternative was a `.ctl/input.toml` read by the TOML path. It lost on
scope: a second file for the same settings, with its own precedence rule,
for a consumer that asks for none.

### 6. The checks prove the split

`check:tree` fails when `--no-default-features --features input-toml`
reaches `serde-saphyr`, `yamled`, or `garde` among normal dependencies.
`check:features` compiles `input-yaml` alone and `input-toml` alone, with
warnings denied. A new size tier, `input-toml` at `app,input-toml`, records
the `.text` CI measures, and the `input` tier moves only if CI measures a
change.

No automatic behaviour is added, so no new override or disable path is
needed.

## Risks

1. A consumer with only `input-toml` that called YAML or garde API stops
   compiling. The changeset names the break and the fix: add `input-yaml` or
   `validate`.
2. A serde-saphyr update could change its defaults. The test in decision 3
   catches it before release.

[limits]: https://github.com/victor-software-house/ctl-core/blob/9141d9f9a5fa5f7b32995945cd0101011f311486/ctl-core/src/input/limits.rs
[toml-text]: https://github.com/victor-software-house/ctl-core/blob/9141d9f9a5fa5f7b32995945cd0101011f311486/ctl-core/src/input/toml_text.rs

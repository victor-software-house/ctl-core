# Design

## Context

[proposal.md](proposal.md) lists eight changes the maintainer approved for 0.7.0.
Each one either lets a type grow in a minor release or closes a field that a caller could misuse.
This design records the choices the list left open.

## Decisions

### 1. An error keeps the I/O kind, not the `io::Error`

`InputError` and `ServeError` gain a private `io_kind: Option<io::ErrorKind>`, read through `io_kind()`.
The error's message still holds the `io::Error` text, as before.

The alternative was to keep the whole `io::Error` and return it from `Error::source`.
It lost for these reasons:

1. `io::Error` is neither `Clone` nor `PartialEq`.
   Both error types derive `Clone`, `PartialEq`, and `Eq`, and tests compare them.
   Keeping the error would need an `Arc<io::Error>` and a hand-written `PartialEq` that ignores it.
   Equality would then skip a field, which the docs must explain.
2. `io::ErrorKind` is `Copy`, `Eq`, and `Hash`, so the derives stay as they are.
3. What a caller acts on is the kind, such as `NotFound` against `PermissionDenied`.
   The OS text is already in the message that `Display` prints.

[C-GOOD-ERR][c-good-err] still holds: both types implement `Error` and `Display`, and are `Send`, `Sync`, and `'static`.
`source()` returns `None`, because no error is kept to return.

### 2. One constructor per error shape

Each error type gets private constructors: one for a refusal with no I/O cause, one for an I/O failure.
Every place that built the struct with a literal calls one of them.
So a new private field changes one constructor, not every call site.

### 3. Open sets and closed sets

An enum is `#[non_exhaustive]` when a plan already names a variant it could gain:

| Enum | Variant it could gain | Source |
|:--|:--|:--|
| `Block` | a panel, a tree, columns | [CTC-035][ledger] |
| `OutputFormat` | YAML | [CTC-043][ledger] |
| `NoticeLevel` | an info level | none yet; the maintainer listed it |

An enum stays exhaustive when its set is closed by what it means, and its rustdoc says why in one sentence.
`Shell` set the pattern.
A consumer's match on a closed enum then needs no wildcard arm, and a new variant fails the build at every match that must handle it.

The alternative was `#[non_exhaustive]` on every public enum.
It lost because a consumer would then write a wildcard arm for a set that cannot grow, and that arm hides nothing but mistakes.

### 4. The colour flags have one reader

`OutputArgs` and `ColorLong` keep `color` and `no_color` private.
`color()` combines them, so `--no-color` always wins.
A caller that read the `color` field directly saw `auto` under `--no-color`.

### 5. `View` reads through methods

`View` already holds every other setting privately behind a setter.
`format` and `quiet` join them: `format()` and `is_quiet()` read them, and `quiet(bool)` stays the setter.
The getter for a boolean takes `is_`, because `quiet` names the setter.
No public `format` setter exists, since `View::new` takes the format.
`App` changes the format of a parse-error view through a crate-private `in_format`.

### 6. `EnvPrefix` is sealed at its public path

`EnvPrefix: sealed::Sealed`, where `sealed` is a private module and `Sealed` is implemented only for `clap::Command`.
The trait keeps its path, `ctl_core::flags::EnvPrefix`, and the prelude export, so `#[command(env_prefix = "...")]` works unchanged.

## Risks

1. Each consumer that builds a struct literal or matches without a wildcard arm changes code when it takes 0.7.0.
   The changeset names each migration.
2. cargo-semver-checks reports each break.
   The `minor` bump allows them while the workspace is below 1.0.

[ledger]: ../../../tasks.yaml
[c-good-err]: https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err

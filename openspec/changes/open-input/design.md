# Design

## Context

[proposal.md](proposal.md) gives the reasons, and [research.md](research.md) the evidence behind each choice.

Declared input lives in [ctl-core/src/input/][input-dir].
Three operations match on the closed `Format` enum: `parse`, `places` and `top_level`.
YAML reads through vsh-serde-saphyr, our fork of serde-saphyr, and places through yamled; TOML reads and places through the `toml` crate's spanned tree.
No consumer enables `input-toml` today, and five consumers enable `input` ([research.md](research.md), section 4).

## Goals / Non-Goals

**Goals:**

1. One data model for every input format, so each operation above it is written once.
2. A format a CLI writes is a peer of the built-ins: same trait, same registry, same placing and messages.
3. YAML rules stay in the fork that owns them; ctl-input copies none of them.

**Non-Goals:**

1. The JSON format and its reader, a separate change.
2. The argument document and `--input` ([CTC-043][ledger]), which build on this change.
3. Writing or editing a text: qctl's edits stay on yamled.
4. Front matter of other formats: `Input::frontmatter` stays YAML.

## Decisions

### 1. A `ctl-input` leaf crate

ctl-input holds the tree, the format trait and registry, the Deserializer, placing, entries, checks, limits and messages.
Its features are `yaml`, `toml` and `validate`; ctl-core's `input-yaml`, `input-toml` and `validate` turn them on, and `ctl_core::input` re-exports the crate.
It releases at ctl-core's version, as ctl-text, ctl-layout and ctl-term do.

The maintainer chose the leaf over keeping the module in ctl-core, and over growing front-matter (operator, 2026-11-26).
front-matter can later read through ctl-input instead of its own three readers; this change does not touch it.

### 2. The located tree

```rust
pub struct Node { span: Range<usize>, used_at: Option<Range<usize>>, tag: Option<Tag>, kind: NodeKind }
pub enum NodeKind {
    Scalar(Scalar),
    Sequence(Vec<Node>),
    Mapping(Vec<(Node, Node)>),
}
pub struct Scalar { value: String, style: ScalarStyle }
```

1. `span` is the node's bytes in the text, quotes and block indicators included, so `text[span]` is its source.
2. `value` is the processed value: escapes, folding and chomping applied, as a `String` field receives it today.
3. `ScalarStyle` is `Plain`, `SingleQuoted`, `DoubleQuoted`, `Literal`, `Folded`, `MultilineBasic` or `MultilineLiteral`.
   A TOML basic string is double quoted, a literal string single quoted, and a TOML number, boolean or datetime plain.
   The two multi-line styles are TOML's `"""` and `'''` strings, which `Entry::written` already tells apart.
4. A mapping keeps its entries in source order, key node and value node.
   So a key's place and its value's place are both known.
5. `used_at` holds the span of the outermost alias that replayed a node, the one written where the value is used.
   Every node beneath a replayed node inherits it, and it is `None` elsewhere (decision 7).

The types are public, read-only, and `#[non_exhaustive]`, so a format outside ctl-input builds them through constructors.

The alternative, erasing serde behind a trait object, lost.
Erased-serde turns the native error into a string, so a YAML problem loses its line and column (operator, 2026-11-26).

Building the YAML tree from granit-parser events inside ctl-input lost too.
It copies the fork's limits, duplicate-key identity and scalar rules into a second home (operator, 2026-11-26).

### 3. The format trait

```rust
pub trait Format: Sync {
    fn name(&self) -> FormatName;
    fn extensions(&self) -> &'static [&'static str];
    fn read(&self, text: &str, limits: &Limits, reading: Reading) -> Result<Node, Refusal>;
    fn resolve<'a>(&self, scalar: &'a Scalar, tag: Option<&Tag>, wanted: Wanted) -> Result<Resolved<'a>, Mismatch>;
}
```

1. `read` turns text into the tree under the limits.
   It refuses a second document and anything past a limit.
   `Reading::Strict` also refuses duplicate keys and merge keys, as a config read does.
   `Reading::Listing` keeps them as ordinary entries, as `top_level` reads a text the caller did not write.
   A `Refusal` names its problem type and span.
2. `resolve` reads one scalar for the type its field needs, with the tag its node carries.
   A YAML tag such as `!!str` changes how a scalar reads.
   `Wanted` is `Any`, `Bool`, `Int`, `Float`, `Str` or `Null`.
   `Resolved` is `Null`, `Bool`, `Int(i128)`, `Uint(u128)`, `Float(f64)` or `Str(&str)`.
3. The trait is object safe: the registry holds `&'static dyn Format`, as the output registry does.

Each format keeps its own scalar rules (operator, 2026-11-26).

YAML resolves through the fork's public scalar rules.
So a quoted `"8"` still fills an integer field, and only the six YAML 1.2 spellings fill a bool.

TOML resolves by syntax: a TOML string fills only a string field, and a TOML integer only a number field, as the `toml` crate refuses today.

The YAML rules stay exactly as the fork applies them today, typeless `010` as float 10.0 included.
Reading it as an integer only in typeless reads would conflict with the typed read, which refuses the leading zero.

### 4. The registry and how an input finds it

`Formats` holds the formats in order, with `builtin()`, `add`, `replace`, `remove`, `get(name)` and `of(path)`, which matches the extension without regard to case.

An input finds its registry through tiers, as `Limits` does: the input's own (`Input::formats`), then the process default (`set_default_formats`), then `Formats::builtin()`.
`App::input_formats` sets the process default before the parse, so `Input::open` in a handler sees the CLI's formats.
`Formats::empty()` with no format added disables reading by extension; `Input::new_with_format` still reads with the format it is given.

### 5. One Deserializer above the tree

ctl-input implements `serde::Deserializer` for a node once.
`deserialize_any` asks the format for `Wanted::Any`; a typed method asks for its own kind; a mismatch becomes a problem at the node's span.
`deserialize_struct` and `deserialize_map` accept only a mapping, so a sequence read into a struct keeps today's `expected a mapping` problem.
`deserialize_option` reads a null-resolving scalar as `None`, and `Option` for a TOML key that is present is always `Some`.

The Deserializer is monomorphized once per target type in ctl-input, and serde-saphyr's own Deserializer is instantiated once for the tree reader.
So a CLI with several config types should link less code than today.
[research.md](research.md), section 5, gives the numbers to confirm.

### 6. YAML through the fork's located reader

A fork patch adds to vsh-serde-saphyr:

1. `located::read(text, &Options) -> Result<LocatedDocument, Error>`: the document as nodes with byte span, processed value, style and tag, each alias expanded.
   It applies the same `Budget`, duplicate-key, merge-key and multiple-document checks as `from_str_with_options`.
2. A public `scalars` module: the YAML 1.2 rules for null, bool, integer and float.
   `deserialize_any` and the typed methods use them today, with the options that select them.

ctl-input's YAML format calls `located::read` and converts its nodes into the tree, and resolves through `scalars`.
The patch is generic serde-saphyr functionality, released as a fork version before ctl-input depends on it.

### 7. Aliases expand in the format's reader

A format with aliases expands each one while it reads, under its own limits, so the tree holds no alias node.
A replayed node keeps the span of the anchored node it copies.
`used_at` holds the span of the outermost alias that brought it, and every node beneath it inherits that span.

A problem in a replayed node sits at `used_at` and names where its value is defined, as today's alias problems do.

The fork's event pipeline already replays aliases under `max_aliases`, `max_events`, the recorded-anchor limits and its replay limits, and refuses an alias inside its own anchor.

Every replayed node counts against the replay limit as it is read, so nested aliases fail before the expanded tree exists.

`Limits` gains the fork's three replay limits under the fork's names: `max_total_replayed_events`, `max_replay_stack_depth` and `max_alias_expansions_per_anchor`.

They follow the same tiers as every other limit, `.ctl/input.yaml` included, and `Loosened` reports a raised one.

They set the fork's `Options::alias_limits`, not its `Budget`.
A breach is the fork's `AliasReplayLimitExceeded`, `AliasReplayStackDepthExceeded` or `AliasExpansionLimitExceeded`, a limit problem by its variant.
A breach names its field and where to raise it, as every limit problem does.

So the YAML limits keep their meaning, and ctl-input counts no aliases of its own.
The alternative, alias nodes that the Deserializer replays, lost: it puts a second set of alias limits in ctl-input.

### 8. Placing, entries and checks walk the tree

`Input::place`, `place_key`, `problem_at`, `check` and `top_level` walk the tree, for every format.

An input keeps the tree it read, as it keeps the yamled document today, and reads again only when the limits in force change.
A strict tree also serves placing and listing, so `load` reads the text once.
A listing tree is read only when no strict tree exists, as for `top_level` on a text no parse has read.

A missing field is placed on the key that holds its mapping, or on line 1 for the root, in every format.

### 9. One message set

Every problem has one of ten problem types.
The problem types are syntax, duplicate key, merge key, expected kind, invalid value, unknown field, unknown variant, missing field, limit and alias.

Invalid value covers a scalar its format cannot resolve for the field, and serde's `invalid_value` and `invalid_length`.
It also covers a `custom` error a type's own `Deserialize` raises, such as a `try_from` newtype.
A custom error keeps the type's own text and sits at the node whose read raised it.

ctl-input words each problem type once, in plain words with no parser option names.
Examples are `duplicate key \`prefix\`` and `expected true or false`.

`Messages` is the trait behind that wording, with an English default.
`set_messages` replaces it for the process.
A problem's type and span stay the same whatever words it gets.
The fork's errors map to these problem types through their variants, never their text.

### 10. `Tagged` belongs to ctl-input

`input::Tagged<T>` reads a node's tag beside its value.
The spelling is the fork's `Tagged` spelling today: `!override` for a local tag, the resolved handle and suffix otherwise.
The value is read from the same node without its tag.

A tag inside a value serde buffers, such as an untagged enum or a flattened field, is not passed on.
The alternative detects serde's private buffer type by name, which a serde release can break.

## Risks / Trade-offs

1. [Wording moves in consumer tests] → each consumer's migration pull request updates its pinned text; the table in [research.md](research.md), section 4, lists every pinned line.
2. [A syntax error now wins over an earlier type error] → the tree is read whole before any type is read.
   A reader fixes syntax first in either order.
3. [Tags inside buffered values are dropped] → hakkiri, the only `Tagged` user, reads tags at the outer level; a test pins that case.
4. [The fork patch is new code in a parser] → it reuses the fork's event pipeline and runs its 4,000-test suite.
   ctl-input's YAML tests run against it before the switch.
5. [Size] → each tier is measured before and after in one box, and budgets move by the measured difference.

## Migration Plan

1. Release the fork patch as a new vsh-serde-saphyr version.
2. Move `ctl-core/src/input/` into `ctl-input` with the tree, keeping every public path under `ctl_core::input`.
3. Consumers move in their 0.8.0 adoption pull requests: `Format::Toml` becomes `&input::TOML`, `Format::of` becomes `Formats::of`, and pinned wording moves.

[input-dir]: https://github.com/victor-software-house/ctl-core/tree/0d8e5a40c6ce8d7d3e1a0d6b1cfc8c836825a7e1/ctl-core/src/input
[ledger]: ../../../tasks.yaml

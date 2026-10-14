# Design: read TOML config files through the declared-input loader

## Decisions

### 1. The `toml` crate's span-carrying tree, not `toml_edit` or `Spanned`

[`toml` 1.1.8][toml-crate] (published 2026-10-09) parses a document into
[`DeTable`][detable]: a map from `Spanned<DeString>` keys to
`Spanned<DeValue>` values, where every span is a byte range into the text.
[`Deserializer::from`][deserializer] reads a `Spanned<DeTable>` into any
`DeserializeOwned` type, and its [`de::Error`][de-error] carries
`message()` and `span()`. One parse therefore gives both halves YAML needs:

1. A parse error and a serde error (type mismatch, missing field, a value
   out of range) each carry a span, which becomes a line and column.
2. The same tree is a location index. A garde field path is walked with
   `DeValue::get` by key or by array index, and the span of the last value
   found is the problem's position.
   `Input::check` parses again to build that index, as it does with yamled
   for YAML, because garde runs on the typed value, not on the tree.

Measured with a throwaway crate against 1.1.8 on a build runner:

| Text | Span start (line, column) |
|:--|:--|
| `name = ''` value | 1, 8 |
| `labels = []` value | 2, 10 |
| `[[outputs]]` second element | its header line |
| `[[outputs]]` second `path` value | the value on its own line |
| inline table `file = { name = "x" }`, `name` value | the value inside the braces |
| `flag = true` then `flag = false`, parse error | 2, 1 (the second key) |
| `[x]` twice, parse error | 2, 2 (the second `x`) |
| `a = [1, 2` with no close, parse error | 1, 10, "unclosed array, expected `]`" |
| `port = 'eighty'` into a `u16`, serde error | 1, 8, the value |
| a missing field | the enclosing table |

Alternatives that lost:

- **[`toml_edit` 0.25.17][toml-edit-crate].**
  `Document::parse` keeps spans too, so it would work.
  It lost because it is the format-preserving editing model, which this
  loader never uses (no CLI edits TOML in place). Its deserializer takes an
  owned `Document<String>`, so keeping the tree for placement needs a clone
  or a second parse, and `indexmap` is a required dependency of it.
  If a CLI ever edits TOML in place, `toml_edit` is the crate for that, as
  yamled is for YAML.
- **`toml::Spanned<T>` fields in each schema.** The span is available per
  field, so no index is needed. It lost because every consumer would change
  its types to `Spanned<String>`, `Spanned<Vec<..>>`, and so on, and garde
  rules would then run on wrappers. The loader's contract is that a schema
  stays a plain serde and garde type, as it is for YAML.
- **`toml_parser` events directly.** It is the lexer and event layer under
  both crates above. It lost because building a tree from its events is
  exactly what `DeTable` already is.
- **Byte offsets in `Problem`.** It lost because `Problem` already speaks
  1-based line and column for YAML, and a consumer renders both the same way.

A position is the span's start, counted in characters, the way
`toml::de::Error` prints its own. The span of a value is its text: `''`
for an empty string. A problem on a value lands on the value, as the YAML
placement does.

Override: none, because a position is not a policy. Disable: not applicable.

### 2. The format is chosen, never guessed

`Format` is `Yaml` or `Toml`. `Input::open(path)` reads a file and takes its
format from the extension, compared without regard to case: `.toml` is TOML,
`.yaml` and `.yml` are YAML. Any other extension, or none, is refused with a
problem naming the extension and the accepted ones, before the file is read.
`Format::of(path)` is that rule on its own, for a CLI that reads the file
itself. `Input::format(Format)` sets the format of text in memory.

`Input::new` and `Input::read` stay YAML. Every current consumer reads that
way, with names such as `tasks.yaml` and `.changeset/a.md`, and
`Input::frontmatter` reads YAML out of Markdown. Their extension says nothing
about the text's format.

Alternatives that lost:

- **Sniffing the text.** `name = "release"` is a valid YAML plain scalar,
  and a TOML table header `[a]` is a valid YAML flow sequence. Either guess
  is wrong for some real file, and the wrong parser reports a misleading
  problem.
- **Making `Input::read` choose by extension.** It would refuse `.md` and
  extensionless names that YAML consumers read today.
- **An extension-only rule with no explicit setter.** Text in memory, such as
  a test fixture or an embedded default config, has no extension.

Override: `Input::format` beats the extension, and `Input::new` plus
`Input::read` never consult it. Disable: call `Input::read` or `Input::new`
instead of `Input::open`.

`Input::frontmatter` is Markdown's `---` YAML block, so the input it returns
is YAML whatever the Markdown input's format. A `---` line is not TOML.

### 3. `input-toml` is additive on top of `input`

`input` keeps its name and meaning: YAML, `validate`, and the shared limits.
`input-toml = ["input", "dep:toml"]` adds the TOML path. A YAML-only consumer
compiles no TOML crate, and an existing `features = ["input"]` builds
unchanged. `Format::Toml` and the TOML code exist only with the feature, and
every `match` on `Format` inside ctl-core is exhaustive for each feature set.
`Format` is `#[non_exhaustive]`: Cargo unifies features across a build, so
one crate turning on `input-toml` would otherwise break another crate's
exhaustive `match` written against `input` alone.

Alternative that lost: split the feature into `input-yaml` and `input-toml`,
with `input` as an alias for the first, so a TOML-only consumer compiles no
serde-saphyr or yamled. It lost for now because `Limits` is a view of
serde-saphyr's `Budget`: `Limits::budget()` returns one, its defaults are
serde-saphyr's, and `REPO_FILE` is YAML. Splitting means giving `Limits` its
own defaults and a second repository-file reader, which changes YAML
behaviour for no consumer that needs it yet. Revisit it when a TOML-only
consumer cares about the compile cost.

The first TOML consumer therefore also compiles serde-saphyr, and the
repository limits file stays `.ctl/input.yaml` for a TOML-config repository.

Dependency footprint, from `cargo tree` on the locked graph, recorded in the
tasks evidence: the `toml` crate with `std`, `parse`, and `serde` brings
`toml_parser`, `toml_datetime`, `serde_spanned`, `winnow`, and `serde_core`.
`display`, `fast_hash`, and `preserve_order` stay off, so no `toml_writer`,
`indexmap`, or `foldhash`.

### 4. Limits: three fields apply, the rest have no TOML meaning

`Limits` is one set of tiers (call, repository, process, default). A TOML
text reads three fields from it:

| Field | TOML meaning |
|:--|:--|
| `max_reader_input_bytes` | The text's size, checked before parsing, with the same message as YAML |
| `max_depth` | Nesting of tables and arrays; the root table is 1 |
| `max_nodes` | The root table, plus every table, array, and value |

Every other field is about YAML syntax and is ignored for TOML:
`max_buffered_comment_events`, `simple_key_max_lookahead`,
`flow_nesting_limit`, `max_events`, `max_aliases`, `max_anchors`,
`max_recorded_anchor_events`, `max_recorded_anchor_bytes`,
`max_property_expansion_depth`, `max_total_property_interpolation_work`,
`max_inclusion_depth`, `max_documents` (TOML has one document),
`max_total_scalar_bytes` (bounded by the byte limit),
`max_total_comment_bytes`, `max_merge_keys`, `alias_anchor_min_aliases`, and
`alias_anchor_ratio_multiplier`. TOML has no anchors, aliases, merge keys,
or `!include`. A duplicate key is a parse error in TOML itself, so there is
no duplicate-key policy to set, and it reports on the second key's line.

`max_depth` and `max_nodes` are checked on the parsed tree, not while
parsing as serde-saphyr's budget does, so a hostile text is held in memory
before either is refused. `max_reader_input_bytes` bounds that memory, and
the `toml` parser bounds its own recursion at 80 levels (a fixed constant,
removed only by its `unbounded` feature, which ctl-core does not enable). A
text nested past 80 fails inside the parser with its wording ("cannot
recurse further; max recursion depth met", or "recursion limit" with no
position for a dotted header), and `max_depth` cannot raise that.

The walk visits table entries in the order they were written, so the problem
for `max_nodes` names the value that crossed the limit.

Alternative that lost: count with `toml_parser` events while parsing, to
refuse early. It lost because the byte limit already bounds the work, and
the second pass over events is code with no consumer that needs it.

Override: the four tiers above, as for YAML. Disable: set a field to a larger
value in the call, the repository file, or the process default. The parser's
80-level bound has no switch.

### 5. One split of the garde field path

garde names a field `outputs[1].path`. YAML turns that into a yamled path.
The split (a key, then `[n]` as an index or `[name]` as a key) moves into
`field_steps`, and both the YAML and the TOML locators read its steps. A key
that contains `.` or `[` is ambiguous in both, so such a problem lands on the
nearest ancestor that resolves, as it already does for YAML.

Alternative that lost: a second splitter next to `yaml_path`. It lost because
two splitters would disagree on the first odd key.

### 6. A message names the person's fix where the parser gives one

A TOML problem message is the `toml` crate's own wording, without the
snippet and the position it prints, because `Problem` carries the position.
The limit messages are YAML's, and name `.ctl/input.yaml`. An unknown
extension reads `unsupported extension .json; name the file *.toml, *.yaml,
or *.yml`.

## Risks

1. A `toml` patch release may reword a message. Tests assert positions and
   the wording ctl-core writes itself, and only that a parse error has a
   non-empty message.
2. `DeTable` iterates sorted by key unless `preserve_order` is on, so the
   walk sorts entries by key span to get document order.

[toml-crate]: https://crates.io/crates/toml/1.1.8
[toml-edit-crate]: https://crates.io/crates/toml_edit/0.25.17
[detable]: https://docs.rs/toml/1.1.8/toml/de/type.DeTable.html
[deserializer]: https://docs.rs/toml/1.1.8/toml/de/struct.Deserializer.html
[de-error]: https://docs.rs/toml/1.1.8/toml/de/struct.Error.html

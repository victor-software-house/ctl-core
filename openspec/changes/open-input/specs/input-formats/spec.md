## ADDED Requirements

### Requirement: Every format reads its text into one located tree

An input format SHALL read its text into ctl-input's located tree.
In the tree, every node SHALL keep its byte span.
Every scalar SHALL keep its processed value, style and tag, and every mapping its key and value nodes in source order.

Placing a path, listing top-level entries, checking with garde and reading a typed value SHALL each work on the tree.
They SHALL work the same for every format.

#### Scenario: The same field in two formats

- **WHEN** `app.yaml` holds `name: ''` on line 2 and `app.toml` holds `name = ''` on line 2, and a schema refuses an empty name
- **THEN** `load` fails on each with one problem for `name` on line 2, worded the same

### Requirement: A CLI adds an input format

`input::Format` SHALL be a public, object-safe trait with a name and the file extensions it reads.
It SHALL have a `read` from text to the tree under the limits in force.
It SHALL have a `resolve` of one scalar for the type the field needs.

A format a CLI writes SHALL be read, placed, listed and checked exactly as the built-in formats are.

#### Scenario: A CLI's own format

- **WHEN** a CLI registers a format named `kv` that reads `.kv` files of `key=value` lines, and `app.kv` holds `port=x` on line 3
- **THEN** `Input::open` reads `app.kv`, and loading it into a type with a `port: u16` field fails with one problem on line 3 for `port`

### Requirement: The registry is found through tiers

`input::Formats` SHALL hold formats in order, with `builtin()` holding `yaml` and `toml` as their features allow, and `add`, `replace`, `remove`, `get` and `of(path)`.
An input SHALL take its registry from `Input::formats`, else the process default set by `set_default_formats`, else `Formats::builtin()`.
`App::input_formats` SHALL set the process default before the command runs.

#### Scenario: A process default

- **WHEN** a CLI calls `set_default_formats(Formats::builtin().remove("toml"))` and then `Input::open("app.toml")`
- **THEN** it fails with one problem that names `.toml` as unsupported and lists `*.yaml` and `*.yml`

#### Scenario: An empty registry

- **WHEN** an input's registry is `Formats::empty()`
- **THEN** `Input::open` refuses every extension, and `Input::new_with_format` still reads with the format it is given

### Requirement: Each format resolves its own scalars

A typed field SHALL have the input's format resolve each scalar for the type the field needs.

YAML SHALL resolve by vsh-serde-saphyr's rules.
Only `true` and `false` in any of their three spellings fill a bool.
A quoted number fills a number field.

TOML SHALL resolve by syntax.
A TOML string fills only a string field, and a TOML number only a number field.

#### Scenario: A quoted number in YAML

- **WHEN** a YAML text holds `port: "8080"` and the field is `u16`
- **THEN** the field reads 8080

#### Scenario: A quoted number in TOML

- **WHEN** a TOML text holds `port = "8080"` on line 1 and the field is `u16`
- **THEN** loading fails with one problem on line 1, column 8, for `port`

#### Scenario: A YAML 1.1 boolean

- **WHEN** a YAML text holds `enabled: yes` and the field is `bool`
- **THEN** loading fails with one problem for `enabled` that reads `expected true or false`

### Requirement: An alias expands in the format's reader

A format with aliases SHALL expand each alias while it reads, under its own limits, so the tree SHALL hold no alias node.
A replayed node SHALL keep the span of the node it copies and the span of the alias that brought it.

A problem in a replayed node SHALL sit at the alias and name where its value is defined.

#### Scenario: An alias past the limit

- **WHEN** a YAML text holds three aliases and `max_aliases` is 2
- **THEN** reading it fails with one problem that names `max_aliases`

#### Scenario: A bad value through an alias

- **WHEN** a YAML text defines `&port x` on line 1 and uses `*port` on line 4 for a `u16` field
- **THEN** loading fails with one problem on line 4 that names line 1 as where the value is defined

### Requirement: Tagged reads a tag beside its value

`input::Tagged<T>` SHALL read a node's tag and its value read without the tag, with a local tag spelled as written, such as `!override`.
A tag inside a value serde buffers, such as an untagged enum or a flattened field, SHALL not be passed on.

#### Scenario: An override tag

- **WHEN** a YAML text holds `code_span_size: !override 0` and the field is `Tagged<u64>`
- **THEN** the field reads value 0 with the tag `!override`

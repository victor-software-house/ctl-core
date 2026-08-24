# Unified ctl presentation

## Decision

Typed domain data is the source of every command presentation. ctl-core owns
the Rust CLI chassis and rendering framework once; consumer CLIs own domain
models and domain-specific information hierarchy.

The public boundary is semantic:

```text
Clap graph ──→ App ──→ domain handler ──→ Result model
                                              │
                                   ┌──────────┴──────────┐
                                   │                     │
                             serde JSON             Present
                                                         │
                                                     Document
                                                         │
                                             pretty or colorless
```

## Ownership

ctl-core owns:

- Usage mounts, optional consumer enrichment through `App::usage_spec`,
  pre-parse short-circuits, help, Clap parsing, exit codes, and stream selection;
- `OutputArgs`, `FormatArgs`, `FormatLong`, `ColorLong`, `ColorMode`, `OutputFormat`, and quiet behavior;
- `Document`, `Text`, `Fields`, `Table`, `Section`, `Notice`, `Rule`, and
  unwrapped verbatim blocks that remove terminal controls, bidi controls, and
  Unicode line separators while preserving other Unicode formatting;
- semantic roles and the one visual theme;
- width detection, wrapping (including Usage lines), table layout, ANSI policy,
  stdout, and stderr;
- JSON emission and generic error envelopes.

A consumer owns:

- Clap-derived domain verbs and parameters;
- serializable request, result, notice, and domain error types;
- the adapter that maps one result to semantic document nodes;
- operator prose in its skill and installed instructions.

A domain handler never prints, constructs a terminal table, reads terminal
width, or selects pretty versus JSON.

## Engine boundary

Comfy Table is a private Rust table and wrapping engine. Anstyle and Anstream
are private style and stream engines. Their types cannot appear in ctl-core's
public semantic API or in a consumer crate.

This boundary is deliberate. Comfy Table is mature, width-aware, and tested.
The broad Rust ports of Python Rich remain young and API-unstable. The facade
lets ctl-core replace an engine later without changing domain models or
consumer render adapters.

Do not implement Unicode width, ANSI stripping, table sizing, or terminal
capability detection from scratch.

## Output law

One serializable model feeds all modes:

1. JSON serializes the model directly, writes one newline to stdout, and never
   contains ANSI.
2. Pretty asks `Present` for a semantic document and renders it with color
   policy `auto` or `always`.
3. Colorless renders the same document with color policy `never`; it does not
   maintain a second layout.
4. Human errors use stderr. JSON failures use stdout so machine consumers read
   one stream.
5. Quiet suppresses successful human output only. It never hides errors or
   JSON.

## Help law

Clap derive types are the command grammar. ctl-core extracts help from that
graph and renders it through the same document renderer. No consumer maintains
a second help parameter model or a local terminal-width implementation.

## Fluent boundary

Consumer presentation code composes meaning:

```rust,ignore
Document::new()
    .heading("status")
    .fields(
        Fields::new()
            .row("active", result.active.as_deref().unwrap_or("none"))
            .row("queued", result.queued.to_string()),
    )
    .table(
        Table::new(["id", "title"])
            .token_column(0)
            .row([result.id.as_str(), result.title.as_str()]),
    )
    .notice(Notice::new(NoticeLevel::Warning, "one stale row"))
```

It does not choose borders, spacing, ANSI styles, width, or streams.

## Operator surface

Clap is also the source for committed operator documents. `Surface` extracts
binary and mounted names, package metadata, recursive commands, aliases,
visibility, locally declared argument and flag metadata, and mounted Usage KDL.
Ancestor globals stay normalized on their declaring command and appear
separately as each descendant's `inherited_arguments`. Templates can render the
effective flag set without losing declaration provenance. It records hidden
commands for verification while the shared command-inventory fragment renders
visible commands only. `Surface::note` carries optional skill- or
instruction-specific prose when Clap's `about` addresses another audience.

ctl-core ships three MiniJinja fragments:

- `ctl/version.md.jinja` renders a skill frontmatter version line;
- `ctl/invocation.md.jinja` renders mounted examples and the no-`--` rule;
- `ctl/commands.md.jinja` renders the visible top-level command inventory.

A consumer template imports those fragments and keeps its own domain prose.
Tests render with the consumer's Clap type and byte-compare the result with the
committed skill or installed instructions. Adding or renaming a Clap command
therefore changes `Surface` and fails the committed-render test until the
operator document moves with it. No token scan or copied command list remains.

## Migration order

1. Publish the ctl-core presentation kernel.
2. Migrate verctl, which already has typed report models and uses ctl-core
   `View`.
3. Migrate forkctl, retaining its protocol types while deleting local help,
   view, layout, and terminal dependencies.
4. Migrate qctl handlers from direct printing to typed results, then add shared
   pretty, colorless, and JSON views.
5. Pin the released `surface` feature in each consumer, render its skill and
   instructions from shared fragments plus domain prose, and delete copied verb
   scans and invocation rules.

Each consumer pins a released ctl-core version. No path dependencies connect
repositories.

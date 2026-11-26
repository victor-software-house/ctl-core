# Proposal

## Motivation

ctl-core's JSON error envelope carries one `message` string for every failure.
Three failures put several parts into that one string:

1. A parse error with clap's `error-context` on joins the message, a tip paragraph, the Usage line and a pointer to `--help`.
2. A refused input joins one line per problem, each with its file, line, column and field.
3. A handler error joins its whole `anyhow` chain with `: `.

A caller that pipes `--format json` to `jq` then parses prose to find the argument, the line or the cause.
JSON output keeps a field per part and never flattens one into a string (operator, 2026-11-26).

## Changes

1. `ErrorBody` gains optional fields, each left out of JSON when empty: `causes`, `problems`, `usage` and `parse`.
2. A parse error fills `parse` from clap: the reason and every part of clap's error context, each kind in its own field, as far as the build's features provide them.
3. ctl-core renders `usage` itself for every parse error, so it is there with or without `error-context`.
4. A refused input fills `problems`, one object per problem, and its `message` names the input and the number of problems.
5. A handler error's `message` is the outermost error, and `causes` lists the rest of its chain in order.
6. `message` is one sentence for every failure.
7. `ErrorBody` also keeps the text a person reads, out of JSON, so `pretty` prints exactly what it prints today.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `presentation`: the error envelope's fields and how `pretty` renders them.

## Impact

1. Code: `ctl-core/src/model.rs`, `ctl-core/src/app.rs` (the error paths), `ctl-core/src/format/interface.rs` (the default `write_error`).
2. Consumers: `ErrorBody` is already `#[non_exhaustive]`, so its new fields break no build; a script that read `message` for a problem's line or a cause reads `problems` or `causes`.
3. Release: a `ctl-core: patch` changeset in the 0.8.0 candidates, naming the shorter JSON `message`.
4. Size: the `app` tier and those above it move by the measured difference.
5. [CTC-043][ledger]'s input document reports its problems and placed clap errors through these fields.

[ledger]: ../../../tasks.yaml

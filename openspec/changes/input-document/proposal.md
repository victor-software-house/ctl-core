# Proposal

## Motivation

A ctl CLI takes a command's arguments only as flags and their environment variables.
Long flag lists are hard to write and to review, as adding rows to a work queue with qctl showed (operator, 2026-11-07).
Output already offers several formats; input offers one (operator, 2026-11-07).

[CTC-043][ledger] asks for one structured document per command, read from a file or stdin, in any input format.
The maintainer fixed the shape on 2026-11-25:

1. The flags are `--input FILE` or `--input -` and `--input-format NAME`, with no short forms, read from `<PREFIX>_INPUT` and `<PREFIX>_INPUT_FORMAT`.
2. The document's top-level keys are the selected command's long flag names and positional names.
3. Clap's own parsers and rules check every value, and an unknown key fails with its file and line.
4. Precedence is the command line, then the environment, then the document, then the default.
5. `App` adds the flags to every command when `app` and an input format feature are both on, and a CLI removes them with one builder call.

Clap 4.6.6 has no layer between the environment and the default ([research.md](research.md), section 1).
A default value skips clap's relational checks, and a synthetic variable needs `unsafe` code.
The maintainer chose synthesized arguments (operator, 2026-11-26).

## Changes

1. `App` adds `--input` and `--input-format` to every command when `app` and any `input-*` feature are on; `App::without_input()` removes them.
2. The document is read through `ctl_core::input`, so every format the registry holds works, a CLI's own included.
3. Each key the command line and the environment left unset becomes argument tokens at the selected command's level, and clap parses the result once.
4. A key that conflicts with a higher layer is dropped by default; `App::input_conflicts` makes it fail instead (operator, 2026-11-26).
5. Every problem in the document names its file, line and column, clap's errors included.
   For that, the input features turn on clap's `error-context` when `cli` is on (operator, 2026-11-26).
6. The document supplies positionals only when the command line gives none at that level (operator, 2026-11-26).

## Capabilities

### New Capabilities

- `input-document`: the input flags, the document's keys, how they join the command line, and how a problem is placed.

### Modified Capabilities

None.

## Impact

1. Code: a new `ctl-core/src/app/document.rs` and its parts; `App` gains `without_input` and `input_conflicts`.
2. Features: `input-yaml`, `input-toml` and `input-json` turn on `clap?/error-context`; `check:tree` allows it with an input feature and still refuses it without one.
3. Size: the `input`, `input-toml` and `input-json` tiers move by the measured difference; `app` and `usage` stay.
4. Consumers: none change until they adopt the release; CTC-043's golden tests in qctl, verctl and forkctl land with each adoption.
5. Release: a `ctl-core: patch` changeset in the 0.8.0 candidates, since every addition is new.

[ledger]: ../../../tasks.yaml

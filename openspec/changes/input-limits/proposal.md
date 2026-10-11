# Proposal: input limits

Queue row: CTC-008 (forkctl onto the shared reader).

## Why

forkctl parses its manifest with its own serde-saphyr options so that a
hostile or broken file cannot exhaust it: 2 MiB of text, one document, depth
32, and 100,000 nodes. `ctl_core::input` carries only serde-saphyr's defaults
(1,024 documents, depth 64, 250,000 nodes, 64 MiB of scalar text, and no cap
on in-memory text). forkctl moves onto the shared reader, so those limits move
with it, and every tool gets them.

## What Changes

1. `Input::parse` refuses a text over 2 MiB before parsing, as a problem with
   no line.
2. It parses with a budget of one document, depth 32, 100,000 nodes, and 2 MiB
   of scalar text.
3. The limits are public constants: `input::MAX_BYTES`, `MAX_DEPTH`, and
   `MAX_NODES`. There is no per-tool override.
4. Unchanged: only `true` and `false` are booleans, duplicate keys and merge
   keys are refused, and anchors and aliases are read under serde-saphyr's
   alias-to-anchor ratio guard.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `declared-input`: the reader enforces safety limits.

## Impact

1. A patch release of ctl-core. A file that exceeds a limit is now refused.
   The largest ledger or config any ctl repository carries today is far under
   each limit.
2. forkctl drops its own parser and reads through `Input::parse`.
